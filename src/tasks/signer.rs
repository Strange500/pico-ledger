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

#[embassy_executor::task]
pub async fn signer_task(mut trng: Trng<'static, TRNG>) -> ! {
    let mut encrypted_wallet: Option<(Vec<u8>, [u8; 12])> = None;
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
                let mut encryption_key = [0u8; 32];

                pbkdf2::pbkdf2::<Hmac<Sha256>>(
                    password.join("").as_bytes(),
                    b"pico-ledger-salt",
                    100_000,
                    &mut encryption_key,
                )
                .expect("Failed to derive encryption key");
                encryption_key.zeroize();

                let mut nonce_bytes = [0u8; 12];
                trng.fill_bytes(&mut nonce_bytes).await;
                let nonce = Nonce::from(nonce_bytes);
                nonce_bytes.zeroize();

                let cipher = ChaCha20Poly1305::new(&encryption_key.into());
                let ciphertext = cipher
                    .encrypt(&nonce, entropy.as_ref())
                    .expect("Encryption failed");

                encrypted_wallet = Some((ciphertext, nonce_bytes));
                nonce_bytes.zeroize();
                entropy.zeroize();
                encryption_key.zeroize();
            }
        }
    }
}
