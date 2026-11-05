#![cfg(feature = "psbt-demo")]
use bitcoin::{psbt::Psbt, secp256k1::Secp256k1, Amount, EcdsaSighashType};
use pico_ledger_core::demo::{sign_demo, UNSIGNED_PSBT};

#[test]
fn fixed_demo_produces_a_real_signature_and_preserves_transaction() {
    let unsigned = Psbt::deserialize(UNSIGNED_PSBT).unwrap();
    let mut randomness = [42; 32];
    let signed = Psbt::deserialize(&sign_demo(&mut randomness).unwrap()).unwrap();
    assert_eq!(randomness, [0; 32]);
    let mut expected = unsigned.clone();
    expected.inputs[0].partial_sigs = signed.inputs[0].partial_sigs.clone();
    assert_eq!(signed, expected);
    assert_eq!(signed.inputs[0].partial_sigs.len(), 1);
    let (key, signature) = signed.inputs[0].partial_sigs.iter().next().unwrap();
    let hash = bitcoin::sighash::SighashCache::new(&unsigned.unsigned_tx)
        .p2wpkh_signature_hash(
            0,
            &unsigned.inputs[0]
                .witness_utxo
                .as_ref()
                .unwrap()
                .script_pubkey,
            Amount::from_sat(100_000),
            EcdsaSighashType::All,
        )
        .unwrap();
    Secp256k1::verification_only()
        .verify_ecdsa(&hash.into(), &signature.signature, &key.inner)
        .unwrap();
    assert_eq!(signature.sighash_type, EcdsaSighashType::All);
    // Signing randomness blinds the operation, not the deterministic ECDSA result.
    assert_eq!(sign_demo(&mut [43; 32]).unwrap(), signed.serialize());
    let mut demo = pico_ledger_core::wallet::Wallet::default();
    demo.generate(
        &mut [0; 32],
        &["1", "2", "3", "4", "5", "6", "7", "8"],
        [0; 12],
    )
    .unwrap();
    let review = demo
        .review_psbt(&mut [44; 32], &signed.serialize())
        .unwrap();
    assert_eq!(review.fee_sat, 1_000);
    assert_eq!(review.outputs[0].amount_sat, 99_000);
    assert!(!review.outputs[0].is_change);
    // No actual UTXO: fabricated funding tx has a null coinbase-like input.
    assert!(
        unsigned.inputs[0].non_witness_utxo.as_ref().unwrap().input[0]
            .previous_output
            .is_null()
    );
}
