use bitcoin::{
    absolute::LockTime,
    address::NetworkUnchecked,
    bip32::{ChildNumber, DerivationPath, Fingerprint, KeySource, Xpub},
    consensus::deserialize,
    key::Secp256k1,
    psbt::Psbt,
    transaction::Version,
    Address, Amount, EcdsaSighashType, Network, NetworkKind, OutPoint, PublicKey, ScriptBuf,
    Sequence, Transaction, TxIn, TxOut, Witness,
};
use std::{error::Error, str::FromStr};

pub struct BuildRequest<'a> {
    pub descriptor: &'a str,
    pub previous_transaction_hex: &'a str,
    pub vout: u32,
    pub input_index: u32,
    pub recipient: &'a str,
    pub amount_sat: u64,
    pub fee_sat: u64,
    pub change_index: u32,
}

struct AccountDescriptor {
    fingerprint: Fingerprint,
    account: Xpub,
}

fn parse_descriptor(descriptor: &str) -> Result<AccountDescriptor, Box<dyn Error>> {
    let descriptor = descriptor.trim();
    let key = descriptor
        .strip_prefix("wpkh([")
        .and_then(|value| value.strip_suffix("/<0;1>/*)"))
        .ok_or("Unsupported descriptor format")?;
    let (origin, account) = key.split_once(']').ok_or("Missing key origin")?;
    let (fingerprint, path) = origin.split_once('/').ok_or("Missing origin path")?;
    if path != "84h/1h/0h" {
        return Err("Only testnet BIP84 account zero is supported".into());
    }
    let fingerprint = Fingerprint::from_str(fingerprint)?;
    let account = Xpub::from_str(account)?;
    if account.network != NetworkKind::Test || account.depth != 3 {
        return Err("Descriptor does not contain a testnet account xpub".into());
    }
    Ok(AccountDescriptor {
        fingerprint,
        account,
    })
}

fn decode_hex(text: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    let text = text.trim();
    if !text.len().is_multiple_of(2) {
        return Err("Previous transaction hex has odd length".into());
    }
    text.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            let pair = core::str::from_utf8(pair)?;
            Ok(u8::from_str_radix(pair, 16)?)
        })
        .collect()
}

fn full_path(chain: u32, index: u32) -> Result<DerivationPath, Box<dyn Error>> {
    Ok(DerivationPath::from(vec![
        ChildNumber::from_hardened_idx(84)?,
        ChildNumber::from_hardened_idx(1)?,
        ChildNumber::from_hardened_idx(0)?,
        ChildNumber::from_normal_idx(chain)?,
        ChildNumber::from_normal_idx(index)?,
    ]))
}

fn derived_key(
    descriptor: &AccountDescriptor,
    chain: u32,
    index: u32,
) -> Result<(bitcoin::secp256k1::PublicKey, ScriptBuf, KeySource), Box<dyn Error>> {
    let secp = Secp256k1::verification_only();
    let relative_path = DerivationPath::from(vec![
        ChildNumber::from_normal_idx(chain)?,
        ChildNumber::from_normal_idx(index)?,
    ]);
    let public_key = descriptor
        .account
        .derive_pub(&secp, &relative_path)?
        .public_key;
    let script = ScriptBuf::new_p2wpkh(&PublicKey::new(public_key).wpubkey_hash()?);
    Ok((
        public_key,
        script,
        (descriptor.fingerprint, full_path(chain, index)?),
    ))
}

pub fn address(descriptor: &str, chain: u32, index: u32) -> Result<String, Box<dyn Error>> {
    if chain > 1 {
        return Err("Chain must be 0 for receive or 1 for change".into());
    }
    let descriptor = parse_descriptor(descriptor)?;
    let (_, script, _) = derived_key(&descriptor, chain, index)?;
    Ok(Address::from_script(&script, Network::Testnet)?.to_string())
}

pub fn build(request: &BuildRequest<'_>) -> Result<Vec<u8>, Box<dyn Error>> {
    let descriptor = parse_descriptor(request.descriptor)?;
    let previous: Transaction = deserialize(&decode_hex(request.previous_transaction_hex)?)?;
    let previous_output = previous
        .output
        .get(usize::try_from(request.vout)?)
        .ok_or("Previous output index is out of bounds")?
        .clone();
    let (input_key, input_script, input_source) = derived_key(&descriptor, 0, request.input_index)?;
    if previous_output.script_pubkey != input_script {
        return Err("Previous output does not belong to the descriptor index".into());
    }

    let recipient = Address::<NetworkUnchecked>::from_str(request.recipient)?
        .require_network(Network::Testnet)?;
    if !recipient.script_pubkey().is_p2wpkh() {
        return Err("Recipient must be a testnet P2WPKH address".into());
    }
    if Amount::from_sat(request.amount_sat) < recipient.script_pubkey().minimal_non_dust() {
        return Err("Recipient amount would be dust".into());
    }
    let spend = request
        .amount_sat
        .checked_add(request.fee_sat)
        .ok_or("Amount overflow")?;
    let change_sat = previous_output
        .value
        .to_sat()
        .checked_sub(spend)
        .ok_or("Amount and fee exceed the input")?;

    let mut outputs = vec![TxOut {
        value: Amount::from_sat(request.amount_sat),
        script_pubkey: recipient.script_pubkey(),
    }];
    let change = if change_sat == 0 {
        None
    } else {
        let (key, script, source) = derived_key(&descriptor, 1, request.change_index)?;
        if Amount::from_sat(change_sat) < script.minimal_non_dust() {
            return Err("Change would be dust".into());
        }
        outputs.push(TxOut {
            value: Amount::from_sat(change_sat),
            script_pubkey: script,
        });
        Some((key, source))
    };

    let unsigned = Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint {
                txid: previous.compute_txid(),
                vout: request.vout,
            },
            script_sig: ScriptBuf::new(),
            sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
            witness: Witness::new(),
        }],
        output: outputs,
    };
    let mut psbt = Psbt::from_unsigned_tx(unsigned)?;
    psbt.inputs[0].non_witness_utxo = Some(previous);
    psbt.inputs[0].witness_utxo = Some(previous_output);
    psbt.inputs[0].sighash_type = Some(EcdsaSighashType::All.into());
    psbt.inputs[0]
        .bip32_derivation
        .insert(input_key, input_source);
    if let Some((key, source)) = change {
        psbt.outputs[1].bip32_derivation.insert(key, source);
    }
    Ok(psbt.serialize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_policy_compatible_psbt() {
        let password = ["1", "2", "3", "4", "5", "6", "7", "8"];
        let mut wallet = pico_ledger_core::wallet::Wallet::default();
        wallet.generate(&mut [0; 32], &password, [1; 12]).unwrap();
        let descriptor = wallet.account_descriptor(&mut [9; 32]).unwrap();
        let parsed_descriptor = parse_descriptor(&descriptor).unwrap();
        let (_, input_script, _) = derived_key(&parsed_descriptor, 0, 0).unwrap();
        let previous = Transaction {
            version: Version::ONE,
            lock_time: LockTime::ZERO,
            input: vec![TxIn {
                previous_output: OutPoint::null(),
                script_sig: ScriptBuf::new(),
                sequence: Sequence::MAX,
                witness: Witness::new(),
            }],
            output: vec![TxOut {
                value: Amount::from_sat(100_000),
                script_pubkey: input_script.clone(),
            }],
        };
        let (_, recipient_script, _) = derived_key(&parsed_descriptor, 0, 9).unwrap();
        let recipient = Address::from_script(&recipient_script, Network::Testnet).unwrap();
        let previous_hex = bitcoin::consensus::encode::serialize_hex(&previous);
        let bytes = build(&BuildRequest {
            descriptor: &descriptor,
            previous_transaction_hex: &previous_hex,
            vout: 0,
            input_index: 0,
            recipient: &recipient.to_string(),
            amount_sat: 90_000,
            fee_sat: 1_000,
            change_index: 0,
        })
        .unwrap();
        let psbt = Psbt::deserialize(&bytes).unwrap();
        assert_eq!(psbt.inputs[0].non_witness_utxo, Some(previous));
        assert_eq!(psbt.unsigned_tx.output[0].value, Amount::from_sat(90_000));
        assert_eq!(psbt.unsigned_tx.output[1].value, Amount::from_sat(9_000));
        assert_eq!(psbt.inputs[0].bip32_derivation.len(), 1);
        assert_eq!(psbt.outputs[1].bip32_derivation.len(), 1);

        let review = wallet.review_psbt(&mut [2; 32], &bytes).unwrap();
        assert_eq!(review.fee_sat, 1_000);
        assert_eq!(review.outputs.len(), 2);
        assert!(review.outputs[1].is_change);
        assert!(wallet.sign_psbt(&mut [3; 32], &bytes, |_| true).is_ok());
        assert_eq!(
            address(&descriptor, 0, 0).unwrap(),
            Address::from_script(&input_script, Network::Testnet)
                .unwrap()
                .to_string()
        );
    }
}
