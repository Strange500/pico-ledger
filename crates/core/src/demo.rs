//! TEST ONLY: a closed signing demo using public keys, never the user's wallet.
//! There is deliberately no API accepting PSBT bytes, a wallet, or an approval flag.
use crate::wallet::Wallet;
use alloc::vec::Vec;
use zeroize::Zeroizing;

pub const UNSIGNED_PSBT: &[u8] = include_bytes!("../../../fixtures/demo.psbt");

/// Sign only the compiled-in, unfunded testnet fixture with a separate public seed.
/// Caller supplies fresh TRNG randomness for secp blinding. Never fund this key.
/// This is NOT an implementation of production transaction review/authorization.
pub fn sign_demo(randomness: &mut [u8; 32]) -> Result<Vec<u8>, &'static str> {
    let mut randomness = Zeroizing::new(core::mem::take(randomness));
    let mut demo = Wallet::default();
    // Public all-zero BIP39 entropy and public BIP39 passphrase "12345678".
    // Fixed nonce is only for this disposable, identical public encryption key/plaintext.
    demo.generate(
        &mut [0; 32],
        &["1", "2", "3", "4", "5", "6", "7", "8"],
        [0; 12],
    )?;
    demo.sign_psbt(&mut randomness, UNSIGNED_PSBT, |review| {
        review.input_count == 1
            && review.outputs.len() == 1
            && review.total_input_sat == 100_000
            && review.total_output_sat == 99_000
            && review.fee_sat == 1_000
    })
}
