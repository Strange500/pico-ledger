//! Regenerate the public, unfunded fixture. Run on a host, without psbt-demo.
use bitcoin::{
    absolute::LockTime,
    bip32::{DerivationPath, Xpriv},
    key::Secp256k1,
    psbt::Psbt,
    transaction::Version,
    Amount, Network, OutPoint, PublicKey, ScriptBuf, Transaction, TxIn, TxOut,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let secp = Secp256k1::new();
    let seed = bip39::Mnemonic::from_entropy(&[0; 32])
        .map_err(|e| format!("{e}"))?
        .to_seed_normalized("12345678");
    let master = Xpriv::new_master(Network::Testnet, &seed).map_err(|e| format!("{e}"))?;
    let path: DerivationPath = "m/84'/1'/0'/0/0".parse().map_err(|e| format!("{e}"))?;
    let key = master
        .derive_priv(&secp, &path)
        .map_err(|e| format!("{e}"))?
        .private_key
        .public_key(&secp);
    let script = ScriptBuf::new_p2wpkh(
        &PublicKey::new(key)
            .wpubkey_hash()
            .map_err(|e| format!("{e}"))?,
    );
    // Deliberately fabricated coinbase-like funding tx; not a chain UTXO.
    let previous = Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint::null(),
            ..TxIn::default()
        }],
        output: vec![TxOut {
            value: Amount::from_sat(100_000),
            script_pubkey: script.clone(),
        }],
    };
    let mut psbt = Psbt::from_unsigned_tx(Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint {
                txid: previous.compute_txid(),
                vout: 0,
            },
            ..TxIn::default()
        }],
        output: vec![TxOut {
            value: Amount::from_sat(99_000),
            script_pubkey: script,
        }],
    })
    .map_err(|e| format!("{e}"))?;
    psbt.inputs[0].non_witness_utxo = Some(previous.clone());
    psbt.inputs[0].witness_utxo = Some(previous.output[0].clone());
    psbt.inputs[0]
        .bip32_derivation
        .insert(key, (master.fingerprint(&secp), path));
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures");
    std::fs::create_dir_all(&dir).map_err(|e| format!("{e}"))?;
    let bytes = psbt.serialize();
    std::fs::write(dir.join("demo.psbt"), &bytes).map_err(|e| format!("{e}"))?;
    println!(
        "Public TEST ONLY fixture: {} bytes, txid {}",
        bytes.len(),
        psbt.unsigned_tx.compute_txid()
    );
    Ok(())
}
