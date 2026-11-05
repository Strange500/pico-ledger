use alloc::vec::Vec;
use bip39::Mnemonic;
use bitcoin::bip32::Xpriv;
use chacha20poly1305::aead::Aead;
use chacha20poly1305::KeyInit;
use chacha20poly1305::{ChaCha20Poly1305, Nonce};
use sha2::Sha256;
use zeroize::{Zeroize, Zeroizing};

static SALT: &[u8] = b"pico-ledger-salt";
static NETWORK: bitcoin::network::Network = bitcoin::network::Network::Testnet;

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
    fn master_key(&self) -> Result<Xpriv, &'static str> {
        let seed = self.seed.as_ref().ok_or("Wallet locked")?;
        Xpriv::new_master(NETWORK, seed.as_ref()).map_err(|_| "Invalid master key")
    }

    pub fn review_psbt(
        &self,
        bytes: &[u8],
    ) -> Result<crate::psbt::TransactionReview, &'static str> {
        Ok(crate::psbt::validate(bytes, &self.master_key()?)?.review)
    }

    /// Validates before calling the trusted approver with the exact transaction review.
    /// No signature is produced on rejection. The immutable PSBT cannot be swapped after
    /// approval. This callback is NOT a physical-presence proof: only trusted device UI
    /// code may implement it. Firmware signing remains disconnected until that UI exists.
    pub fn sign_psbt(
        &self,
        bytes: &[u8],
        approve: impl FnOnce(&crate::psbt::TransactionReview) -> bool,
    ) -> Result<Vec<u8>, &'static str> {
        let master = self.master_key()?;
        let validated = crate::psbt::validate(bytes, &master)?;
        if !approve(&validated.review) {
            return Err("Transaction rejected");
        }
        validated.sign(&master)
    }
}
