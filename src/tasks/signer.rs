use crate::signals::{Message, SignerMessage, MESSAGE_QUEUE, SIGNER_CHANNEL};
use embassy_rp::peripherals::TRNG;
use embassy_rp::trng::Trng;
use pico_ledger_core::wallet::Wallet;

#[embassy_executor::task]
pub async fn signer_task(mut trng: Trng<'static, TRNG>) -> ! {
    let mut wallet = Wallet::default();
    loop {
        match SIGNER_CHANNEL.receive().await {
            SignerMessage::GenerateSeed { password } => {
                if wallet.is_initialized() {
                    continue;
                }
                let mut entropy = [0; 32];
                let mut nonce = [0; 12];
                trng.fill_bytes(&mut entropy).await;
                trng.fill_bytes(&mut nonce).await;
                match wallet.generate(&mut entropy, &password, nonce) {
                    Ok(words) => {
                        MESSAGE_QUEUE
                            .send(Message::SeedPhraseGenerated(words))
                            .await
                    }
                    Err(err) => log_error!("Seed generation failed: {}", err),
                }
            }
            SignerMessage::Lock => wallet.lock(),
            SignerMessage::CheckPassword { password } => {
                MESSAGE_QUEUE
                    .send(Message::PasswordChecked(wallet.unlock(&password).is_ok()))
                    .await;
            }
            SignerMessage::Sign {
                tx_hash,
                address_index,
            } => match wallet.sign(tx_hash, address_index) {
                Ok(signature) => {
                    MESSAGE_QUEUE
                        .send(Message::TransactionSigned { signature })
                        .await
                }
                Err(err) => log_error!("Signing failed: {}", err),
            },
        }
    }
}
