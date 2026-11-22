use alloc::{format, string::String, vec, vec::Vec};
use bip39::Mnemonic;
use bitcoin::bip32::{ChildNumber, DerivationPath, Xpriv, Xpub};
use bitcoin::secp256k1::{All, Secp256k1};
use chacha20poly1305::aead::Aead;
use chacha20poly1305::KeyInit;
use chacha20poly1305::{ChaCha20Poly1305, Nonce};
use sha2::Sha256;
use zeroize::{Zeroize, Zeroizing};

static SALT: &[u8] = b"pico-ledger-salt";
static NETWORK: bitcoin::network::Network = bitcoin::network::Network::Testnet;

/// Public, deterministic entropy used only by the explicitly unsafe firmware test mode.
pub const TEST_MODE_ENTROPY: [u8; 32] = [0; 32];
/// Public passphrase/PIN used only by the explicitly unsafe firmware test mode.
pub const TEST_MODE_PASSWORD: [&str; 8] = ["0"; 8];

/// Owns a backend key without exposing Copy/Clone. Cleanup also runs on early return.
/// rust-bitcoin/secp256k1 internal copies and compiler spills remain outside this guard.
pub(crate) struct SecretXpriv(pub(crate) Xpriv);

impl Zeroize for SecretXpriv {
    fn zeroize(&mut self) {
        self.0.private_key.non_secure_erase();
        <bitcoin::bip32::ChainCode as AsMut<[u8]>>::as_mut(&mut self.0.chain_code).zeroize();
    }
}

impl Drop for SecretXpriv {
    fn drop(&mut self) {
        self.zeroize();
    }
}

fn randomized_context(randomness: &mut [u8; 32]) -> Secp256k1<All> {
    let seed = Zeroizing::new(*randomness);
    randomness.zeroize();
    let mut secp = Secp256k1::new();
    secp.seeded_randomize(&seed);
    secp
}

#[cfg(test)]
mod secret_tests {
    use super::*;

    #[test]
    fn owned_extended_key_cleanup_erases_secret_and_chain_code() {
        let mut key = SecretXpriv(Xpriv::new_master(NETWORK, &[42; 64]).unwrap());
        key.zeroize();
        // secp256k1 intentionally overwrites SecretKey with ones, a valid scalar.
        assert_eq!(key.0.private_key.secret_bytes(), [1; 32]);
        assert_eq!(&key.0.chain_code[..], &[0; 32]);
        // Drop repeats the same cleanup; explicit erasure is idempotent.
        key.zeroize();
        assert_eq!(key.0.private_key.secret_bytes(), [1; 32]);
        assert_eq!(&key.0.chain_code[..], &[0; 32]);
    }
}

pub fn derive_encryption_key(password: &[&'static str; 8]) -> [u8; 32] {
    let password = Zeroizing::new(password.join(""));
    pbkdf2::pbkdf2_hmac_array::<Sha256, 32>(password.as_bytes(), SALT, 100_000)
}

/// Encrypt entropy with a caller-supplied nonce, which must never repeat for the same key.
pub fn encrypt_entropy(
    entropy: &[u8; 32],
    password: &[&'static str; 8],
    nonce_bytes: [u8; 12],
) -> Result<(Vec<u8>, [u8; 12]), &'static str> {
    let encryption_key = Zeroizing::new(derive_encryption_key(password));
    let nonce = Nonce::from(nonce_bytes);

    let cipher = ChaCha20Poly1305::new(&(*encryption_key).into());
    cipher
        .encrypt(&nonce, entropy.as_ref())
        .map(|ciphertext| (ciphertext, nonce_bytes))
        .map_err(|_| "Encryption failed")
}

pub fn decrypt_entropy(
    ciphertext: &[u8],
    password: &[&'static str; 8],
    nonce_bytes: &[u8; 12],
) -> Result<[u8; 32], &'static str> {
    let encryption_key = Zeroizing::new(derive_encryption_key(password));
    let nonce = Nonce::from(*nonce_bytes);

    let cipher = ChaCha20Poly1305::new(&(*encryption_key).into());
    match cipher.decrypt(&nonce, ciphertext) {
        Ok(plaintext) => {
            let plaintext = Zeroizing::new(plaintext);
            let mut entropy = [0u8; 32];
            if plaintext.len() != entropy.len() {
                return Err("Invalid entropy length");
            }
            entropy.copy_from_slice(&plaintext);
            Ok(entropy)
        }
        Err(_) => Err("Decryption failed"),
    }
}

/// Wallet state shared by firmware and host tests. Random inputs come from the caller.
#[derive(Default)]
pub struct Wallet {
    encrypted: Option<(Vec<u8>, [u8; 12])>,
    seed: Option<Zeroizing<[u8; 64]>>,
}

/// A validated transaction held between on-device review and physical approval.
pub struct PreparedPsbt(crate::psbt::ValidatedPsbt);

impl PreparedPsbt {
    #[must_use]
    pub fn review(&self) -> &crate::psbt::TransactionReview {
        &self.0.review
    }

    /// Signs the exact PSBT that produced the displayed review.
    pub fn sign(self, wallet: &Wallet, randomness: &mut [u8; 32]) -> Result<Vec<u8>, &'static str> {
        let secp = randomized_context(randomness);
        let master = wallet.master_key()?;
        self.0.sign(&master.0, &secp)
    }
}
impl Drop for Wallet {
    fn drop(&mut self) {
        self.lock();
    }
}
impl Wallet {
    pub fn is_initialized(&self) -> bool {
        self.encrypted.is_some()
    }
    /// Consumes and clears the caller's entropy. The nonce must be fresh for each encryption key.
    pub fn generate(
        &mut self,
        entropy: &mut [u8; 32],
        password: &[&'static str; 8],
        nonce: [u8; 12],
    ) -> Result<[&'static str; 24], &'static str> {
        let secret_entropy = Zeroizing::new(*entropy);
        entropy.zeroize();
        if self.is_initialized() {
            return Err("Wallet already initialized");
        }
        let mnemonic =
            Mnemonic::from_entropy(secret_entropy.as_ref()).map_err(|_| "Invalid entropy")?;
        let words = mnemonic
            .words()
            .collect::<Vec<_>>()
            .try_into()
            .map_err(|_| "Invalid word count")?;
        let encrypted = encrypt_entropy(&secret_entropy, password, nonce)?;
        self.seed = Some(Zeroizing::new(
            mnemonic.to_seed_normalized(&Zeroizing::new(password.join(""))),
        ));
        self.encrypted = Some(encrypted);
        Ok(words)
    }
    pub fn lock(&mut self) {
        self.seed = None;
    }
    pub fn unlock(&mut self, password: &[&'static str; 8]) -> Result<(), &'static str> {
        let (ciphertext, nonce) = self.encrypted.as_ref().ok_or("Wallet not initialized")?;
        let mut entropy = decrypt_entropy(ciphertext, password, nonce)?;
        let mnemonic = Mnemonic::from_entropy(&entropy).map_err(|_| "Invalid entropy");
        entropy.zeroize();
        let seed = mnemonic?.to_seed_normalized(&Zeroizing::new(password.join("")));
        self.lock();
        self.seed = Some(Zeroizing::new(seed));
        Ok(())
    }
    fn master_key(&self) -> Result<SecretXpriv, &'static str> {
        let seed = self.seed.as_ref().ok_or("Wallet locked")?;
        Xpriv::new_master(NETWORK, seed.as_ref())
            .map(SecretXpriv)
            .map_err(|_| "Invalid master key")
    }

    /// Returns a watch-only testnet BIP84 multipath descriptor.
    pub fn account_descriptor(&self, randomness: &mut [u8; 32]) -> Result<String, &'static str> {
        let secp = randomized_context(randomness);
        let master = self.master_key()?;
        let account_path = DerivationPath::from(vec![
            ChildNumber::from_hardened_idx(84).map_err(|_| "Invalid purpose")?,
            ChildNumber::from_hardened_idx(1).map_err(|_| "Invalid coin type")?,
            ChildNumber::from_hardened_idx(0).map_err(|_| "Invalid account")?,
        ]);
        let account = SecretXpriv(
            master
                .0
                .derive_priv(&secp, &account_path)
                .map_err(|_| "Account derivation failed")?,
        );
        let fingerprint = master.0.fingerprint(&secp);
        let account_public = Xpub::from_priv(&secp, &account.0);
        Ok(format!(
            "wpkh([{fingerprint}/84h/1h/0h]{account_public}/<0;1>/*)"
        ))
    }

    /// Requires fresh CSPRNG/TRNG bytes for side-channel blinding, not host PSBT data
    /// or a fixed seed. Consumes and clears randomness even on failure.
    pub fn review_psbt(
        &self,
        randomness: &mut [u8; 32],
        bytes: &[u8],
    ) -> Result<crate::psbt::TransactionReview, &'static str> {
        let secp = randomized_context(randomness);
        let master = self.master_key()?;
        Ok(crate::psbt::validate(bytes, &master.0, &secp)?.review)
    }

    /// Validates a PSBT and retains that exact transaction for later device approval.
    pub fn prepare_psbt(
        &self,
        randomness: &mut [u8; 32],
        bytes: &[u8],
    ) -> Result<PreparedPsbt, &'static str> {
        let secp = randomized_context(randomness);
        let master = self.master_key()?;
        crate::psbt::validate(bytes, &master.0, &secp).map(PreparedPsbt)
    }

    /// Validates before calling the trusted approver with the exact transaction review.
    /// No signature is produced on rejection. The immutable PSBT cannot be swapped after
    /// approval. This callback is NOT a physical-presence proof: only trusted device UI
    /// code may implement it. Firmware signing remains disconnected until that UI exists.
    /// Requires fresh CSPRNG/TRNG randomness as in `review_psbt`; always clears it.
    pub fn sign_psbt(
        &self,
        randomness: &mut [u8; 32],
        bytes: &[u8],
        approve: impl FnOnce(&crate::psbt::TransactionReview) -> bool,
    ) -> Result<Vec<u8>, &'static str> {
        let secp = randomized_context(randomness);
        let master = self.master_key()?;
        let validated = crate::psbt::validate(bytes, &master.0, &secp)?;
        if !approve(&validated.review) {
            return Err("Transaction rejected");
        }
        validated.sign(&master.0, &secp)
    }
}
