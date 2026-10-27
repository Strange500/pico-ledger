use core::str::FromStr;

use alloc::format;
use alloc::vec::Vec;
use bip39::Mnemonic;
use bitcoin::bip32::{DerivationPath, Xpriv};
use bitcoin::key::Secp256k1;
use bitcoin::secp256k1::Message as SecpMessage;
use chacha20poly1305::aead::Aead;
use chacha20poly1305::{ChaCha20Poly1305, Nonce};
use embassy_rp::peripherals::TRNG;
use embassy_rp::trng::Trng;
use hmac::{Hmac, KeyInit};
use sha2::Sha256;
use zeroize::Zeroize;

use crate::signals::{Message, SignerMessage, MESSAGE_QUEUE, SIGNER_CHANNEL};

static SALT: &[u8] = b"pico-ledger-salt";
static NETWORK: bitcoin::network::Network = bitcoin::network::Network::Testnet;

fn derive_encryption_key(password: &[&'static str; 8]) -> [u8; 32] {
    let mut encryption_key = [0u8; 32];
    match pbkdf2::pbkdf2::<Hmac<Sha256>>(
        password.join("").as_bytes(),
        SALT,
        100_000,
        &mut encryption_key,
    ) {
        Ok(_) => encryption_key,
        Err(_) => {
            log_error!("Failed to derive encryption key");
            [0u8; 32]
        }
    }
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
    match cipher.encrypt(&nonce, entropy.as_ref()) {
        Ok(ciphertext) => (ciphertext, nonce_bytes),
        Err(_) => {
            log_error!("Encryption failed");
            (Vec::new(), nonce_bytes)
        }
    }
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

    // store master extended private key in memory after decryption for signing
    let mut master_private: Option<[u8; 64]> = None;
    let mut pswr: Option<[&'static str; 8]> = None;

    // Initialize the crypto context once
    let secp = Secp256k1::signing_only();
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
                let Ok(mnemonic) = Mnemonic::from_entropy(&entropy) else {
                    log_error!("Failed to generate mnemonic from entropy");
                    continue;
                };
                // convert into [&'static str; 24] for sending through the channel, but handles the Result properly
                let Ok(words): Result<[&'static str; 24], _> =
                    mnemonic.words().collect::<Vec<_>>().try_into()
                else {
                    log_info!("Failed to convert mnemonic words into array");
                    continue;
                };
                MESSAGE_QUEUE
                    .send(Message::SeedPhraseGenerated(words))
                    .await;
                let (ciphertext, nonce_bytes) =
                    encrypt_entropy(&entropy, &password, &mut trng).await;

                encrypted_wallet = Some((ciphertext, nonce_bytes));
                pswr = Some(password);
                master_private = Some(mnemonic.to_seed_normalized(&password.join("")));
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
            SignerMessage::Sign {
                tx_hash,
                address_index,
            } => {
                if let Some(master_private) = &master_private {
                    let Ok(master_sxpriv) = Xpriv::new_master(NETWORK, master_private) else {
                        log_error!("Failed to create master extended private key");
                        continue;
                    };
                    let path_str = format!("m/84'/0'/0'/0/{}", address_index);
                    let Ok(path) = DerivationPath::from_str(&path_str) else {
                        log_error!("Failed to create derivation path");
                        continue;
                    };

                    let Ok(child_xpriv) = master_sxpriv.derive_priv(&secp, &path) else {
                        log_error!("Failed to derive child extended private key");
                        continue;
                    };
                    let secret_key = child_xpriv.private_key;

                    let message = SecpMessage::from_digest(tx_hash);
                    let sig = secp.sign_ecdsa(&message, &secret_key);

                    log::info!("Generated signature: {:?}", sig.serialize_compact());

                    MESSAGE_QUEUE
                        .send(Message::TransactionSigned {
                            signature: sig.serialize_compact(),
                        })
                        .await;
                }
            }
        }
    }
}
