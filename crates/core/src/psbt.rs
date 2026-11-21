//! Conservative PSBT-v0 / native P2WPKH policy. No transport or physical UI lives here.
use crate::wallet::SecretXpriv;
use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use bitcoin::{
    bip32::{ChildNumber, DerivationPath, KeySource, Xpriv},
    consensus::encode::{Decodable, VarInt},
    ecdsa,
    key::Secp256k1,
    psbt::Psbt,
    secp256k1::{self, Message},
    sighash::{EcdsaSighashType, SighashCache},
    Address, Network, PublicKey, ScriptBuf, Txid,
};

pub const MAX_PSBT_BYTES: usize = 16_384;
pub const MAX_INPUTS: usize = 8;
pub const MAX_OUTPUTS: usize = 8;
const MAX_MONEY: u64 = 21_000_000 * 100_000_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewedOutput {
    pub address: String,
    pub amount_sat: u64,
    /// True only for a verified m/84'/1'/0'/1/index output.
    pub is_change: bool,
}

/// All fields must be displayed by a trusted UI before accepting a transaction.
/// Addresses are testnet; amounts and fees are integer satoshis, never floats.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransactionReview {
    pub txid: Txid,
    pub version: i32,
    pub lock_time: u32,
    pub input_count: usize,
    pub sequences: Vec<u32>,
    pub total_input_sat: u64,
    pub total_output_sat: u64,
    pub fee_sat: u64,
    pub outputs: Vec<ReviewedOutput>,
}

pub(crate) struct ValidatedPsbt {
    psbt: Psbt,
    pub review: TransactionReview,
    unsigned_inputs: Vec<UnsignedInput>,
}

struct UnsignedInput {
    index: usize,
    key: secp256k1::PublicKey,
    path: DerivationPath,
    message: Message,
}

fn checked_amount(total: u64, amount: u64) -> Result<u64, &'static str> {
    total
        .checked_add(amount)
        .filter(|&n| n <= MAX_MONEY)
        .ok_or("Amount out of range")
}

fn owned_path(
    master: &Xpriv,
    secp: &Secp256k1<secp256k1::All>,
    key: &secp256k1::PublicKey,
    (fingerprint, path): &KeySource,
    change_only: bool,
) -> Result<ScriptBuf, &'static str> {
    use ChildNumber::{Hardened as H, Normal as N};
    match path.as_ref() {
        [H { index: 84 }, H { index: 1 }, H { index: 0 }, N { index: chain }, N { .. }]
            if *chain <= 1 && (!change_only || *chain == 1) => {}
        _ => return Err("Unsupported BIP84 path (testnet account zero only)"),
    }
    if *fingerprint != master.fingerprint(secp) {
        return Err("Wrong master fingerprint");
    }
    let derived = SecretXpriv(
        master
            .derive_priv(secp, path)
            .map_err(|_| "Key derivation failed")?,
    );
    if derived.0.private_key.public_key(secp) != *key {
        return Err("Origin does not match derived key");
    }
    Ok(ScriptBuf::new_p2wpkh(
        &PublicKey::new(*key)
            .wpubkey_hash()
            .map_err(|_| "Uncompressed key")?,
    ))
}

pub(crate) fn validate(
    bytes: &[u8],
    master: &Xpriv,
    secp: &Secp256k1<secp256k1::All>,
) -> Result<ValidatedPsbt, &'static str> {
    preflight(bytes)?;
    let mut reader = bytes;
    let psbt = Psbt::deserialize_from_reader(&mut reader).map_err(|_| "Invalid PSBT")?;
    if !reader.is_empty() {
        return Err("Trailing PSBT data");
    }
    let tx = &psbt.unsigned_tx;
    // Reserve the maximum serialized partial-signature entry per input so that a
    // successfully returned PSBT remains importable under the same byte budget.
    let missing_signatures = psbt
        .inputs
        .iter()
        .filter(|input| input.partial_sigs.is_empty())
        .count();
    if bytes.len() + missing_signatures * 109 > MAX_PSBT_BYTES {
        return Err("Insufficient PSBT signature space");
    }
    if tx.version.0 != 1 && tx.version.0 != 2 {
        return Err("Unsupported transaction version");
    }
    let mut unsigned_inputs = Vec::new();
    let mut total_input_sat = 0;
    let mut cache = SighashCache::new(tx);
    for (i, (txin, input)) in tx.input.iter().zip(&psbt.inputs).enumerate() {
        if txin.previous_output.is_null()
            || tx.input[..i]
                .iter()
                .any(|other| other.previous_output == txin.previous_output)
        {
            return Err("Null or duplicate outpoint");
        }
        if input
            .sighash_type
            .is_some_and(|t| t != EcdsaSighashType::All.into())
        {
            return Err("Only SIGHASH_ALL supported");
        }
        // Mandatory even for SegWit: authenticates every input amount against its txid,
        // preventing a hostile coordinator from lying about the total fee across inputs.
        let prev = input
            .non_witness_utxo
            .as_ref()
            .ok_or("Missing non-witness UTXO")?;
        if prev.compute_txid() != txin.previous_output.txid {
            return Err("UTXO txid mismatch");
        }
        let utxo = prev
            .output
            .get(txin.previous_output.vout as usize)
            .ok_or("UTXO vout out of bounds")?;
        if input.witness_utxo.as_ref().is_some_and(|w| w != utxo) {
            return Err("Conflicting witness UTXO");
        }
        if input.bip32_derivation.len() != 1 {
            return Err("Exactly one owned origin required per input");
        }
        let (key, origin) = input
            .bip32_derivation
            .iter()
            .next()
            .ok_or("Missing origin")?;
        if owned_path(master, secp, key, origin, false)? != utxo.script_pubkey {
            return Err("UTXO script is not owned P2WPKH");
        }
        total_input_sat = checked_amount(total_input_sat, utxo.value.to_sat())?;
        let digest = cache
            .p2wpkh_signature_hash(i, &utxo.script_pubkey, utxo.value, EcdsaSighashType::All)
            .map_err(|_| "Invalid P2WPKH sighash")?;
        let message = Message::from(digest);
        for (existing_key, sig) in &input.partial_sigs {
            if *existing_key != PublicKey::new(*key) || sig.sighash_type != EcdsaSighashType::All {
                return Err("Unexpected partial signature");
            }
            Secp256k1::verification_only()
                .verify_ecdsa(&message, &sig.signature, key)
                .map_err(|_| "Invalid existing signature")?;
        }
        if input.partial_sigs.is_empty() {
            unsigned_inputs.push(UnsignedInput {
                index: i,
                key: *key,
                path: origin.1.clone(),
                message,
            });
        }
    }
    let mut outputs = Vec::new();
    let mut total_output_sat = 0;
    for (txout, output) in tx.output.iter().zip(&psbt.outputs) {
        if !txout.script_pubkey.is_p2wpkh() {
            return Err("Only P2WPKH outputs supported");
        }
        let is_change = match output.bip32_derivation.len() {
            0 => false,
            1 => {
                let (key, origin) = output
                    .bip32_derivation
                    .iter()
                    .next()
                    .ok_or("Missing output origin")?;
                if owned_path(master, secp, key, origin, true)? != txout.script_pubkey {
                    return Err("False change claim");
                }
                true
            }
            _ => return Err("Ambiguous output origin"),
        };
        total_output_sat = checked_amount(total_output_sat, txout.value.to_sat())?;
        outputs.push(ReviewedOutput {
            address: Address::from_script(&txout.script_pubkey, Network::Testnet)
                .map_err(|_| "Invalid output address")?
                .to_string(),
            amount_sat: txout.value.to_sat(),
            is_change,
        });
    }
    let fee_sat = total_input_sat
        .checked_sub(total_output_sat)
        .ok_or("Outputs exceed inputs")?;
    let review = TransactionReview {
        txid: tx.compute_txid(),
        version: tx.version.0,
        lock_time: tx.lock_time.to_consensus_u32(),
        input_count: tx.input.len(),
        sequences: tx
            .input
            .iter()
            .map(|i| i.sequence.to_consensus_u32())
            .collect(),
        total_input_sat,
        total_output_sat,
        fee_sat,
        outputs,
    };
    Ok(ValidatedPsbt {
        psbt,
        review,
        unsigned_inputs,
    })
}

impl ValidatedPsbt {
    /// Only called after approval of self.review; owns the exact, immutable transaction.
    pub(crate) fn sign(
        mut self,
        master: &Xpriv,
        secp: &Secp256k1<secp256k1::All>,
    ) -> Result<Vec<u8>, &'static str> {
        for input in self.unsigned_inputs {
            let child = SecretXpriv(
                master
                    .derive_priv(secp, &input.path)
                    .map_err(|_| "Key derivation failed")?,
            );
            let signature = secp.sign_ecdsa(&input.message, &child.0.private_key);
            drop(child);
            self.psbt.inputs[input.index].partial_sigs.insert(
                PublicKey::new(input.key),
                ecdsa::Signature::sighash_all(signature),
            );
        }
        Ok(self.psbt.serialize())
    }
}

// Allocation-free framing pass before rust-bitcoin allocates from attacker-supplied counts.
// This is also the field allowlist: every map rejects unsupported keys before decoding.
// Bound transaction vector cardinalities as well as the encoded byte size.
struct Cursor<'a>(&'a [u8]);
impl<'a> Cursor<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], &'static str> {
        let (head, rest) = self.0.split_at_checked(n).ok_or("Truncated PSBT")?;
        self.0 = rest;
        Ok(head)
    }
    fn number(&mut self) -> Result<usize, &'static str> {
        let VarInt(n) = VarInt::consensus_decode(&mut self.0).map_err(|_| "Invalid CompactSize")?;
        usize::try_from(n).map_err(|_| "Size overflow")
    }
    fn blob(&mut self) -> Result<&'a [u8], &'static str> {
        let n = self.number()?;
        self.take(n)
    }
    fn map(
        &mut self,
        mut visit: impl FnMut(&[u8], &[u8]) -> Result<(), &'static str>,
    ) -> Result<(), &'static str> {
        loop {
            let key = self.blob()?;
            if key.is_empty() {
                return Ok(());
            }
            let value = self.blob()?;
            visit(key, value)?;
        }
    }
}

fn transaction_shape(
    bytes: &[u8],
    max_in: usize,
    max_out: usize,
) -> Result<(usize, usize), &'static str> {
    let mut c = Cursor(bytes);
    c.take(4)?;
    let mut inputs = c.number()?;
    let witness = inputs == 0;
    if witness {
        if c.take(1)? != [1] {
            return Err("Invalid witness flag");
        }
        inputs = c.number()?;
    }
    if inputs == 0 || inputs > max_in {
        return Err("Input limit exceeded");
    }
    for _ in 0..inputs {
        c.take(36)?;
        c.blob()?;
        c.take(4)?;
    }
    let outputs = c.number()?;
    if outputs == 0 || outputs > max_out {
        return Err("Output limit exceeded");
    }
    for _ in 0..outputs {
        c.take(8)?;
        c.blob()?;
    }
    if witness {
        for _ in 0..inputs {
            let items = c.number()?;
            if items > 32 {
                return Err("Witness item limit exceeded");
            }
            for _ in 0..items {
                c.blob()?;
            }
        }
    }
    c.take(4)?;
    if !c.0.is_empty() {
        return Err("Trailing transaction data");
    }
    Ok((inputs, outputs))
}

fn preflight(bytes: &[u8]) -> Result<(), &'static str> {
    if bytes.len() > MAX_PSBT_BYTES {
        return Err("PSBT byte limit exceeded");
    }
    let mut c = Cursor(bytes);
    if c.take(5)? != b"psbt\xff" {
        return Err("Invalid PSBT magic");
    }
    let mut shape = None;
    c.map(|key, value| {
        match key {
            [0] => {
                if shape.is_some() {
                    return Err("Duplicate unsigned transaction");
                }
                shape = Some(transaction_shape(value, MAX_INPUTS, MAX_OUTPUTS)?);
            }
            [0xfb] if value == [0, 0, 0, 0] => {}
            _ => return Err("Unsupported global field"),
        }
        Ok(())
    })?;
    let (inputs, outputs) = shape.ok_or("Missing unsigned transaction")?;
    for _ in 0..inputs {
        c.map(|key, value| {
            match key {
                [0] => {
                    transaction_shape(value, 32, 32)?;
                }
                [1] => {
                    let mut output = Cursor(value);
                    output.take(8)?;
                    output.blob()?;
                    if !output.0.is_empty() {
                        return Err("Trailing witness UTXO data");
                    }
                }
                [3] if value.len() == 4 => {}
                [2, rest @ ..] if rest.len() == 33 && value.len() <= 73 => {}
                [6, rest @ ..] if rest.len() == 33 && value.len() == 24 => {}
                _ => return Err("Unsupported input field"),
            }
            Ok(())
        })?;
    }
    for _ in 0..outputs {
        c.map(|key, value| match key {
            [2, rest @ ..] if rest.len() == 33 && value.len() == 24 => Ok(()),
            _ => Err("Unsupported output field"),
        })?;
    }
    if !c.0.is_empty() {
        return Err("Trailing PSBT data");
    }
    Ok(())
}
