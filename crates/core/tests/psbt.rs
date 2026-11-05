use bitcoin::{
    absolute::LockTime,
    bip32::{DerivationPath, Xpriv},
    key::Secp256k1,
    psbt::Psbt,
    transaction::Version,
    Amount, Network, OutPoint, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Witness,
};
use pico_ledger_core::wallet::Wallet;

const PASSWORD: [&str; 8] = ["1", "2", "3", "4", "5", "6", "7", "8"];

#[test]
fn context_entropy_is_consumed_on_success_rejection_and_error() {
    let (mut wallet, psbt) = fixture();
    let bytes = psbt.serialize();
    let mut randomness = [7; 32];
    wallet.review_psbt(&mut randomness, &bytes).unwrap();
    assert_eq!(randomness, [0; 32]);
    randomness.fill(8);
    let first = wallet.sign_psbt(&mut randomness, &bytes, |_| true).unwrap();
    assert_eq!(randomness, [0; 32]);
    randomness.fill(9);
    let second = wallet.sign_psbt(&mut randomness, &bytes, |_| true).unwrap();
    assert_eq!(randomness, [0; 32]);
    assert_eq!(
        first, second,
        "Blinding must not change deterministic signatures"
    );
    randomness.fill(10);
    assert!(wallet
        .sign_psbt(&mut randomness, &bytes, |_| false)
        .is_err());
    assert_eq!(randomness, [0; 32]);
    randomness.fill(11);
    assert!(wallet.review_psbt(&mut randomness, b"invalid").is_err());
    assert_eq!(randomness, [0; 32]);
    wallet.lock();
    randomness.fill(12);
    assert!(wallet
        .sign_psbt(&mut randomness, &bytes, |_| panic!("locked"))
        .is_err());
    assert_eq!(randomness, [0; 32]);
}

fn fixture() -> (Wallet, Psbt) {
    let mut wallet = Wallet::default();
    wallet.generate(&mut [0; 32], &PASSWORD, [1; 12]).unwrap();
    let secp = Secp256k1::new();
    let seed = bip39::Mnemonic::from_entropy(&[0; 32])
        .unwrap()
        .to_seed_normalized("12345678");
    let master = Xpriv::new_master(Network::Testnet, &seed).unwrap();
    let path: DerivationPath = "m/84'/1'/0'/0/0".parse().unwrap();
    let key = master
        .derive_priv(&secp, &path)
        .unwrap()
        .private_key
        .public_key(&secp);
    let script = ScriptBuf::new_p2wpkh(&bitcoin::PublicKey::new(key).wpubkey_hash().unwrap());
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
    let tx = Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint {
                txid: previous.compute_txid(),
                vout: 0,
            },
            sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
            script_sig: ScriptBuf::new(),
            witness: Witness::default(),
        }],
        output: vec![TxOut {
            value: Amount::from_sat(99_000),
            script_pubkey: script,
        }],
    };
    let mut psbt = Psbt::from_unsigned_tx(tx).unwrap();
    psbt.inputs[0].non_witness_utxo = Some(previous.clone());
    psbt.inputs[0].witness_utxo = Some(previous.output[0].clone());
    psbt.inputs[0]
        .bip32_derivation
        .insert(key, (master.fingerprint(&secp), path));
    (wallet, psbt)
}

#[test]
fn validates_reviews_and_signs_real_bip143_digest() {
    let (wallet, psbt) = fixture();
    let bytes = psbt.serialize();
    let review = wallet.review_psbt(&mut [42; 32], &bytes).unwrap();
    assert_eq!(review.fee_sat, 1_000);
    assert_eq!(review.total_input_sat, 100_000);
    assert_eq!(review.total_output_sat, 99_000);
    assert_eq!(review.outputs.len(), 1);
    assert!(!review.outputs[0].is_change);
    assert!(review.outputs[0].address.starts_with("tb1q"));
    let signed = wallet
        .sign_psbt(&mut [42; 32], &bytes, |exact| exact == &review)
        .unwrap();
    let signed = Psbt::deserialize(&signed).unwrap();
    assert_eq!(signed.unsigned_tx, psbt.unsigned_tx);
    assert_eq!(signed.inputs[0].partial_sigs.len(), 1);
    assert!(signed.inputs[0].final_script_witness.is_none());
    let mut expected = psbt.clone();
    expected.inputs[0].partial_sigs = signed.inputs[0].partial_sigs.clone();
    assert_eq!(signed, expected, "Only partial signatures may change");
    let (key, signature) = signed.inputs[0].partial_sigs.iter().next().unwrap();
    let digest = bitcoin::sighash::SighashCache::new(&psbt.unsigned_tx)
        .p2wpkh_signature_hash(
            0,
            &psbt.inputs[0].witness_utxo.as_ref().unwrap().script_pubkey,
            Amount::from_sat(100_000),
            bitcoin::EcdsaSighashType::All,
        )
        .unwrap();
    Secp256k1::verification_only()
        .verify_ecdsa(
            &bitcoin::secp256k1::Message::from(digest),
            &signature.signature,
            &key.inner,
        )
        .unwrap();
    assert_eq!(signature.sighash_type, bitcoin::EcdsaSighashType::All);
}

#[test]
fn refusal_and_lock_never_sign() {
    let (mut wallet, psbt) = fixture();
    assert!(wallet
        .sign_psbt(&mut [42; 32], &psbt.serialize(), |_| false)
        .is_err());
    wallet.lock();
    assert!(wallet
        .review_psbt(&mut [42; 32], &psbt.serialize())
        .is_err());
    assert!(wallet
        .sign_psbt(&mut [42; 32], &psbt.serialize(), |_| panic!(
            "locked callback"
        ))
        .is_err());
    wallet.unlock(&PASSWORD).unwrap();
    assert!(wallet
        .sign_psbt(&mut [42; 32], &psbt.serialize(), |_| true)
        .is_ok());
}

fn reject(psbt: Psbt, wallet: &Wallet) {
    let bytes = psbt.serialize();
    assert!(wallet.review_psbt(&mut [42; 32], &bytes).is_err());
    assert!(wallet
        .sign_psbt(&mut [42; 32], &bytes, |_| panic!(
            "invalid PSBT reached approval"
        ))
        .is_err());
}

#[test]
fn rejects_bad_utxos() {
    let (wallet, psbt) = fixture();
    let mut p = psbt.clone();
    p.inputs[0].non_witness_utxo = None;
    reject(p, &wallet);
    let mut p = psbt.clone();
    p.inputs[0].witness_utxo.as_mut().unwrap().value = Amount::from_sat(200_000);
    reject(p, &wallet);
    let mut p = psbt.clone();
    p.unsigned_tx.input[0].previous_output.vout = 1;
    reject(p, &wallet);
    let mut p = psbt.clone();
    p.inputs[0].non_witness_utxo.as_mut().unwrap().lock_time = LockTime::from_consensus(1);
    reject(p, &wallet);
    let mut p = psbt;
    p.inputs[0].witness_utxo.as_mut().unwrap().script_pubkey = ScriptBuf::new();
    reject(p, &wallet);
}

#[test]
fn rejects_wrong_origin_path_and_key() {
    let (wallet, psbt) = fixture();
    for path in [
        "m/84'/0'/0'/0/0",
        "m/44'/1'/0'/0/0",
        "m/84'/1'/1'/0/0",
        "m/84'/1'/0'/2/0",
        "m/84'/1'/0'/0/0'",
        "m/84'/1'/0'/0",
        "m/84'/1'/0'/0/1",
    ] {
        let mut p = psbt.clone();
        p.inputs[0].bip32_derivation.values_mut().next().unwrap().1 = path.parse().unwrap();
        reject(p, &wallet);
    }
    let mut p = psbt.clone();
    p.inputs[0].bip32_derivation.values_mut().next().unwrap().0 = [0; 4].into();
    reject(p, &wallet);
    let mut p = psbt;
    p.inputs[0].bip32_derivation.clear();
    reject(p, &wallet);
}

#[test]
fn rejects_non_all_sighash_and_unsupported_scripts() {
    let (wallet, psbt) = fixture();
    for sighash in [0, 2, 3, 0x81, 0x82, 0x83, 0x101] {
        let mut p = psbt.clone();
        p.inputs[0].sighash_type = Some(bitcoin::psbt::PsbtSighashType::from_u32(sighash));
        reject(p, &wallet);
    }
    let mut p = psbt.clone();
    p.inputs[0].redeem_script = Some(ScriptBuf::new());
    reject(p, &wallet);
    let mut p = psbt.clone();
    p.inputs[0].witness_script = Some(ScriptBuf::new());
    reject(p, &wallet);
    let mut p = psbt.clone();
    p.inputs[0].final_script_witness = Some(Witness::default());
    reject(p, &wallet);
    let mut p = psbt;
    p.unsigned_tx.output[0].script_pubkey = ScriptBuf::new();
    reject(p, &wallet);
}

#[test]
fn rejects_negative_fee_duplicates_and_bad_change_claim() {
    let (wallet, psbt) = fixture();
    let mut p = psbt.clone();
    p.unsigned_tx.output[0].value = Amount::from_sat(100_001);
    reject(p, &wallet);
    let mut p = psbt.clone();
    p.unsigned_tx.input.push(p.unsigned_tx.input[0].clone());
    p.inputs.push(p.inputs[0].clone());
    reject(p, &wallet);
    let mut p = psbt;
    p.outputs[0].bip32_derivation = p.inputs[0].bip32_derivation.clone();
    // Receive-chain metadata is not a valid change claim.
    reject(p, &wallet);
}

#[test]
fn rejects_malformed_trailing_oversized_and_non_v0_data() {
    let (wallet, psbt) = fixture();
    for bytes in [vec![], b"not a psbt".to_vec(), vec![0; 16_385]] {
        assert!(wallet.review_psbt(&mut [42; 32], &bytes).is_err());
    }
    let mut bytes = psbt.serialize();
    bytes.push(0);
    assert!(wallet.review_psbt(&mut [42; 32], &bytes).is_err());
    let mut p = psbt;
    p.version = 2;
    reject(p, &wallet);
}

#[test]
fn signs_multiple_inputs_and_verified_change_without_witness_utxo() {
    let (wallet, mut psbt) = fixture();
    let secp = Secp256k1::new();
    let seed = bip39::Mnemonic::from_entropy(&[0; 32])
        .unwrap()
        .to_seed_normalized("12345678");
    let master = Xpriv::new_master(Network::Testnet, &seed).unwrap();
    let change_path: DerivationPath = "m/84'/1'/0'/1/7".parse().unwrap();
    let change_key = master
        .derive_priv(&secp, &change_path)
        .unwrap()
        .private_key
        .public_key(&secp);
    let change_script =
        ScriptBuf::new_p2wpkh(&bitcoin::PublicKey::new(change_key).wpubkey_hash().unwrap());
    psbt.unsigned_tx.output.push(TxOut {
        value: Amount::from_sat(100_000),
        script_pubkey: change_script,
    });
    psbt.outputs.push(bitcoin::psbt::Output::default());
    psbt.outputs[1]
        .bip32_derivation
        .insert(change_key, (master.fingerprint(&secp), change_path));
    let mut second = psbt.inputs[0].clone();
    let previous = second.non_witness_utxo.as_mut().unwrap();
    previous.lock_time = LockTime::from_consensus(1);
    let mut txin = psbt.unsigned_tx.input[0].clone();
    txin.previous_output.txid = previous.compute_txid();
    second.witness_utxo = None;
    psbt.unsigned_tx.input.push(txin);
    psbt.inputs.push(second);
    let review = wallet
        .review_psbt(&mut [42; 32], &psbt.serialize())
        .unwrap();
    assert_eq!(review.input_count, 2);
    assert_eq!(review.fee_sat, 1_000);
    assert!(!review.outputs[0].is_change);
    assert!(review.outputs[1].is_change);
    let signed = wallet
        .sign_psbt(&mut [42; 32], &psbt.serialize(), |r| r == &review)
        .unwrap();
    let parsed = Psbt::deserialize(&signed).unwrap();
    for i in 0..2 {
        assert_eq!(parsed.inputs[i].partial_sigs.len(), 1);
        let (key, sig) = parsed.inputs[i].partial_sigs.iter().next().unwrap();
        let utxo = &parsed.inputs[i].non_witness_utxo.as_ref().unwrap().output[0];
        let digest = bitcoin::sighash::SighashCache::new(&parsed.unsigned_tx)
            .p2wpkh_signature_hash(
                i,
                &utxo.script_pubkey,
                utxo.value,
                bitcoin::EcdsaSighashType::All,
            )
            .unwrap();
        secp.verify_ecdsa(
            &bitcoin::secp256k1::Message::from(digest),
            &sig.signature,
            &key.inner,
        )
        .unwrap();
    }
    assert_eq!(
        wallet.sign_psbt(&mut [42; 32], &signed, |_| true).unwrap(),
        signed
    );
}

#[test]
fn verifies_signature_against_independently_assembled_bip143_preimage() {
    use bitcoin::{
        consensus::serialize,
        hashes::{sha256d, Hash},
    };
    let (wallet, psbt) = fixture();
    let signed = Psbt::deserialize(
        &wallet
            .sign_psbt(&mut [42; 32], &psbt.serialize(), |_| true)
            .unwrap(),
    )
    .unwrap();
    let (key, sig) = signed.inputs[0].partial_sigs.iter().next().unwrap();
    let tx = &psbt.unsigned_tx;
    let mut preimage = serialize(&tx.version);
    let prevouts: Vec<_> = tx
        .input
        .iter()
        .flat_map(|i| serialize(&i.previous_output))
        .collect();
    preimage.extend(sha256d::Hash::hash(&prevouts).to_byte_array());
    let sequences: Vec<_> = tx
        .input
        .iter()
        .flat_map(|i| serialize(&i.sequence))
        .collect();
    preimage.extend(sha256d::Hash::hash(&sequences).to_byte_array());
    preimage.extend(serialize(&tx.input[0].previous_output));
    // BIP143 scriptCode is the serialized P2PKH script, not the witness program.
    preimage.extend(serialize(&ScriptBuf::new_p2pkh(&key.pubkey_hash())));
    preimage.extend(100_000u64.to_le_bytes());
    preimage.extend(serialize(&tx.input[0].sequence));
    let outputs: Vec<_> = tx.output.iter().flat_map(serialize).collect();
    preimage.extend(sha256d::Hash::hash(&outputs).to_byte_array());
    preimage.extend(serialize(&tx.lock_time));
    preimage.extend(1u32.to_le_bytes());
    let digest = sha256d::Hash::hash(&preimage).to_byte_array();
    Secp256k1::verification_only()
        .verify_ecdsa(
            &bitcoin::secp256k1::Message::from_digest(digest),
            &sig.signature,
            &key.inner,
        )
        .unwrap();
}

#[test]
fn never_approves_a_different_transaction_review() {
    let (wallet, mut psbt) = fixture();
    let original = wallet
        .review_psbt(&mut [42; 32], &psbt.serialize())
        .unwrap();
    psbt.unsigned_tx.output[0].value = Amount::from_sat(98_000);
    assert!(wallet
        .sign_psbt(&mut [42; 32], &psbt.serialize(), |r| r == &original)
        .is_err());
}

#[test]
fn rejects_unexpected_metadata_and_invalid_existing_signatures() {
    let (wallet, psbt) = fixture();
    let raw = bitcoin::psbt::raw::Key {
        type_value: 0x50,
        key: vec![],
    };
    let mut p = psbt.clone();
    p.unknown.insert(raw.clone(), vec![1]);
    reject(p, &wallet);
    let mut p = psbt.clone();
    p.inputs[0].unknown.insert(raw.clone(), vec![1]);
    reject(p, &wallet);
    let mut p = psbt.clone();
    p.outputs[0].unknown.insert(raw, vec![1]);
    reject(p, &wallet);
    let mut p = psbt.clone();
    p.inputs[0].tap_internal_key = Some(
        p.inputs[0]
            .bip32_derivation
            .keys()
            .next()
            .unwrap()
            .x_only_public_key()
            .0,
    );
    reject(p, &wallet);
    let signed = wallet
        .sign_psbt(&mut [42; 32], &psbt.serialize(), |_| true)
        .unwrap();
    let mut p = Psbt::deserialize(&signed).unwrap();
    p.unsigned_tx.output[0].value = Amount::from_sat(98_000);
    reject(p, &wallet);
    let mut p = Psbt::deserialize(&signed).unwrap();
    p.inputs[0]
        .partial_sigs
        .values_mut()
        .next()
        .unwrap()
        .sighash_type = bitcoin::EcdsaSighashType::None;
    reject(p, &wallet);
}

#[test]
fn enforces_counts_amount_ranges_and_transactions_without_panics() {
    let (wallet, psbt) = fixture();
    let mut p = psbt.clone();
    p.unsigned_tx.output[0].value = Amount::from_sat(2_100_000_000_000_001);
    reject(p, &wallet);
    let mut p = psbt.clone();
    p.unsigned_tx.input[0].previous_output = OutPoint::null();
    reject(p, &wallet);
    let mut p = psbt.clone();
    p.unsigned_tx.version = Version::non_standard(3);
    reject(p, &wallet);
    let mut p = psbt.clone();
    p.unsigned_tx.output.clear();
    p.outputs.clear();
    reject(p, &wallet);
    let mut p = psbt.clone();
    p.unsigned_tx.input.clear();
    p.inputs.clear();
    reject(p, &wallet);
    let mut p = psbt.clone();
    for _ in 0..8 {
        p.unsigned_tx.output.push(p.unsigned_tx.output[0].clone());
        p.outputs.push(p.outputs[0].clone());
    }
    reject(p, &wallet);
    let mut p = psbt;
    for _ in 0..8 {
        p.unsigned_tx.input.push(p.unsigned_tx.input[0].clone());
        p.inputs.push(p.inputs[0].clone());
    }
    reject(p, &wallet);
}

#[test]
fn all_truncations_duplicate_keys_and_hostile_counts_are_rejected() {
    let (wallet, psbt) = fixture();
    let bytes = psbt.serialize();
    for end in 0..bytes.len() {
        assert!(
            wallet.review_psbt(&mut [42; 32], &bytes[..end]).is_err(),
            "truncation {end}"
        );
    }
    let tx = bitcoin::consensus::serialize(&psbt.unsigned_tx);
    assert!(tx.len() < 253);
    let mut duplicate = bytes.clone();
    let mut entry = vec![1, 0, tx.len() as u8];
    entry.extend(&tx);
    duplicate.splice(5..5, entry);
    assert!(wallet.review_psbt(&mut [42; 32], &duplicate).is_err());
    // Huge counts must be rejected before rust-bitcoin allocates vectors.
    let mut hostile = b"psbt\xff".to_vec();
    let mut tx = 2i32.to_le_bytes().to_vec();
    tx.extend([0xff; 9]);
    hostile.extend([1, 0, tx.len() as u8]);
    hostile.extend(tx);
    hostile.push(0);
    assert!(wallet.review_psbt(&mut [42; 32], &hostile).is_err());
    let mut nonminimal = bytes;
    nonminimal.splice(5..6, [0xfd, 1, 0]);
    assert!(wallet.review_psbt(&mut [42; 32], &nonminimal).is_err());
}

#[test]
fn bounds_previous_transaction_vectors_and_round_trip_signature_space() {
    use pico_ledger_core::psbt::MAX_PSBT_BYTES;
    let (wallet, psbt) = fixture();
    let mut p = psbt.clone();
    let prev = p.inputs[0].non_witness_utxo.as_mut().unwrap();
    for _ in 0..32 {
        prev.output.push(prev.output[0].clone());
    }
    p.unsigned_tx.input[0].previous_output.txid = prev.compute_txid();
    reject(p, &wallet);
    let mut p = psbt.clone();
    let prev = p.inputs[0].non_witness_utxo.as_mut().unwrap();
    prev.input[0].witness = Witness::from_slice(&[[0u8; 0]; 33]);
    reject(p, &wallet);
    let mut p = psbt.clone();
    // Witness data in a previous transaction does not affect its txid.
    p.inputs[0].non_witness_utxo.as_mut().unwrap().input[0].witness =
        Witness::from_slice(&[vec![1]]);
    p.inputs[0].sighash_type = Some(bitcoin::EcdsaSighashType::All.into());
    assert!(wallet
        .sign_psbt(&mut [42; 32], &p.serialize(), |_| true)
        .is_ok());

    let mut p = psbt;
    // Grow a previous input's script to just below the encoded PSBT limit.
    let original_len = p.serialize().len();
    // Both the script and enclosing PSBT value gain two CompactSize bytes.
    let filler = MAX_PSBT_BYTES - original_len - 4 - 110;
    p.inputs[0].non_witness_utxo.as_mut().unwrap().input[0].script_sig =
        ScriptBuf::from_bytes(vec![0; filler]);
    p.unsigned_tx.input[0].previous_output.txid = p.inputs[0]
        .non_witness_utxo
        .as_ref()
        .unwrap()
        .compute_txid();
    let bytes = p.serialize();
    assert_eq!(bytes.len(), MAX_PSBT_BYTES - 110);
    let signed = wallet.sign_psbt(&mut [42; 32], &bytes, |_| true).unwrap();
    assert!(signed.len() <= MAX_PSBT_BYTES);
    assert_eq!(
        wallet.sign_psbt(&mut [42; 32], &signed, |_| true).unwrap(),
        signed
    );
    p.inputs[0].non_witness_utxo.as_mut().unwrap().input[0].script_sig =
        ScriptBuf::from_bytes(vec![0; filler + 2]);
    p.unsigned_tx.input[0].previous_output.txid = p.inputs[0]
        .non_witness_utxo
        .as_ref()
        .unwrap()
        .compute_txid();
    assert_eq!(
        wallet.review_psbt(&mut [42; 32], &p.serialize()),
        Err("Insufficient PSBT signature space")
    );
}
