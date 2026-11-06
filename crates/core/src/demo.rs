//! TEST ONLY: closed review/signing demo, never the user's wallet.
//! No API accepts caller PSBT bytes, keys, or remote approval.
use crate::{psbt::TransactionReview, wallet::Wallet};
use alloc::vec::Vec;
use zeroize::Zeroizing;

pub const UNSIGNED_PSBT: &[u8] = include_bytes!("../../../fixtures/demo.psbt");

fn public_wallet() -> Result<Wallet, &'static str> {
    let mut wallet = Wallet::default();
    // Public entropy, public passphrase, disposable fixed encryption nonce.
    wallet.generate(
        &mut [0; 32],
        &["1", "2", "3", "4", "5", "6", "7", "8"],
        [0; 12],
    )?;
    Ok(wallet)
}

/// A validated review of the immutable compiled-in fixture. Consumed on signing.
/// The UI receives only a clone of the review, never mutable signing bytes.
pub struct DemoReview {
    review: TransactionReview,
}

impl DemoReview {
    /// Validate without producing any signature. Requires fresh TRNG randomness.
    pub fn prepare(randomness: &mut [u8; 32]) -> Result<Self, &'static str> {
        let mut randomness = Zeroizing::new(core::mem::take(randomness));
        let wallet = public_wallet()?;
        Ok(Self {
            review: wallet.review_psbt(&mut randomness, UNSIGNED_PSBT)?,
        })
    }

    pub fn review(&self) -> &TransactionReview {
        &self.review
    }

    /// Called only after the firmware receives physical confirmation. Validate
    /// again, and require exact equality with the complete review that was shown.
    /// Both operations use the SAME immutable bytes, including PSBT metadata.
    pub fn sign(self, randomness: &mut [u8; 32]) -> Result<Vec<u8>, &'static str> {
        let mut randomness = Zeroizing::new(core::mem::take(randomness));
        public_wallet()?.sign_psbt(&mut randomness, UNSIGNED_PSBT, |review| {
            review == &self.review
        })
    }
}

/// Signer-owned single-use sessions. Tokens never repeat across cancel or lock.
#[derive(Default)]
pub struct DemoSession {
    generation: u64,
    pending: Option<(u64, DemoReview)>,
}

impl DemoSession {
    pub fn prepare(
        &mut self,
        randomness: &mut [u8; 32],
    ) -> Result<(u64, TransactionReview), &'static str> {
        let mut randomness = Zeroizing::new(core::mem::take(randomness));
        self.pending = None;
        self.generation = self
            .generation
            .checked_add(1)
            .ok_or("Demo session exhausted")?;
        let prepared = DemoReview::prepare(&mut randomness)?;
        let review = prepared.review().clone();
        self.pending = Some((self.generation, prepared));
        Ok((self.generation, review))
    }
    pub fn cancel(&mut self, token: u64) {
        if self.pending.as_ref().is_some_and(|(id, _)| *id == token) {
            self.pending = None;
        }
    }
    pub fn lock(&mut self) {
        self.pending = None;
    }
    /// Consumes only a matching session, before any signing operation begins.
    pub fn confirm(&mut self, token: u64) -> Option<DemoReview> {
        if !self.pending.as_ref().is_some_and(|(id, _)| *id == token) {
            return None;
        }
        self.pending.take().map(|(_, review)| review)
    }
}

/// Host-only fixture exercise; firmware must use prepare + physical confirmation.
#[cfg(not(target_os = "none"))]
pub fn sign_demo(randomness: &mut [u8; 32]) -> Result<Vec<u8>, &'static str> {
    let mut randomness = Zeroizing::new(core::mem::take(randomness));
    public_wallet()?.sign_psbt(&mut randomness, UNSIGNED_PSBT, |review| {
        review.input_count == 1
            && review.outputs.len() == 1
            && review.total_input_sat == 100_000
            && review.total_output_sat == 99_000
            && review.fee_sat == 1_000
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changed_retained_review_cannot_authorize_signing() {
        let mut pending = DemoReview::prepare(&mut [41; 32]).unwrap();
        pending.review.fee_sat += 1;
        let mut entropy = [42; 32];
        assert_eq!(pending.sign(&mut entropy), Err("Transaction rejected"));
        assert_eq!(entropy, [0; 32]);
    }

    #[test]
    fn replacement_and_token_exhaustion_fail_closed() {
        let mut session = DemoSession::default();
        let (first, _) = session.prepare(&mut [41; 32]).unwrap();
        let (second, _) = session.prepare(&mut [42; 32]).unwrap();
        assert!(session.confirm(first).is_none());
        assert!(session.confirm(second).is_some());
        session.generation = u64::MAX;
        let mut entropy = [43; 32];
        assert_eq!(session.prepare(&mut entropy), Err("Demo session exhausted"));
        assert_eq!(entropy, [0; 32]);
        assert!(session.confirm(second).is_none());
    }
}
