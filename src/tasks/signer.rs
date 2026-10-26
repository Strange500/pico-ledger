use alloc::vec::Vec;
use bip39::Mnemonic;
use chacha20poly1305::aead::Aead;
use chacha20poly1305::{ChaCha20Poly1305, Nonce};
use embassy_rp::peripherals::TRNG;
use embassy_rp::trng::Trng;
use hmac::{Hmac, KeyInit};
use sha2::Sha256;
use zeroize::Zeroize;

use crate::signals::{Message, SignerMessage, MESSAGE_QUEUE, SIGNER_CHANNEL};

static SALT: &[u8] = b"pico-ledger-salt";

fn derive_encryption_key(password: &[&'static str; 8]) -> [u8; 32] {
    let mut encryption_key = [0u8; 32];
    pbkdf2::pbkdf2::<Hmac<Sha256>>(
        password.join("").as_bytes(),
        SALT,
        100_000,
        &mut encryption_key,
    )
    .expect("Failed to derive encryption key");
    encryption_key
}

async fn encrypt_entropy(
    entropy: &[u8; 32],
    password: &[&'static str; 8],
    trng: &mut Trng<'static, TRNG>,
) -> (Vec<u8>, [u8; 12]) {
    let encryption_key = derive_encryption_key(password);
    let mut nonce_bytes = [0u8; 12];
    trng.fill_bytes(&mut nonce_bytes).await;
    let nonce = Nonce::from(nonce_bytes);

    let cipher = ChaCha20Poly1305::new(&encryption_key.into());
    let ciphertext = cipher
        .encrypt(&nonce, entropy.as_ref())
        .expect("Encryption failed");

    (ciphertext, nonce_bytes)
}

fn decrypt_entropy(
    ciphertext: &[u8],
    password: &[&'static str; 8],
    nonce_bytes: &[u8; 12],
) -> Result<[u8; 32], &'static str> {
    let encryption_key = derive_encryption_key(password);
    let nonce = Nonce::from(*nonce_bytes);

    let cipher = ChaCha20Poly1305::new(&encryption_key.into());
    match cipher.decrypt(&nonce, ciphertext) {
        Ok(plaintext) => {
            let mut entropy = [0u8; 32];
            entropy.copy_from_slice(&plaintext);
            Ok(entropy)
        }
        Err(_) => Err("Decryption failed"),
    }
}

#[embassy_executor::task]
pub async fn signer_task(mut trng: Trng<'static, TRNG>) -> ! {
    let mut encrypted_wallet: Option<(Vec<u8>, [u8; 12])> = None;
    let mut pswr: Option<[&'static str; 8]> = None;
    loop {
        let msg = SIGNER_CHANNEL.receive().await;
        match msg {
            SignerMessage::GenerateSeed { password } => {
                if encrypted_wallet.is_some() {
                    log_info!("Seed phrase already generated, skipping generation.");
                    continue;
                }
                let mut entropy = [0u8; 32];
                trng.fill_bytes(&mut entropy).await;
                let mnemonic =
                    Mnemonic::from_entropy(&entropy).expect("Failed to generate seed phrase");
                let words = mnemonic.words().collect::<Vec<_>>();
                MESSAGE_QUEUE
                    .send(Message::SeedPhraseGenerated(words.try_into().unwrap()))
                    .await;

                let (ciphertext, nonce_bytes) =
                    encrypt_entropy(&entropy, &password, &mut trng).await;

                encrypted_wallet = Some((ciphertext, nonce_bytes));
                pswr = Some(password);
                entropy.zeroize();
            }
            SignerMessage::Lock => {
                pswr = None;
                log_info!("Device locked. Password cleared from memory.");
            }
            SignerMessage::CheckPassword { password } => {
                if let Some((ciphertext, nonce_bytes)) = &encrypted_wallet {
                    match decrypt_entropy(ciphertext, &password, nonce_bytes) {
                        Ok(_) => {
                            MESSAGE_QUEUE.send(Message::PasswordChecked(true)).await;
                            pswr = Some(password);
                        }
                        Err(err) => {
                            log_info!("Password check failed: {}", err);
                            MESSAGE_QUEUE.send(Message::PasswordChecked(false)).await;
                        }
                    }
                } else {
                    log_info!("No encrypted wallet found. Generate a seed phrase first.");
                }
            }
            SignerMessage::Test => {
                // try to decrypt with the stored password if it exists
                if let (Some((ciphertext, nonce_bytes)), Some(password)) =
                    (&encrypted_wallet, &pswr)
                {
                    match decrypt_entropy(ciphertext, password, nonce_bytes) {
                        Ok(_) => {
                            log_info!("Test decryption successful.");
                        }
                        Err(err) => {
                            log_info!("Test decryption failed: {}", err);
                        }
                    }
                } else {
                    log_info!("No encrypted wallet or password found.");
                }
            }
        }
    }
}
