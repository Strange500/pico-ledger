use alloc::vec::Vec;
use bip39::Mnemonic;
use embassy_rp::peripherals::TRNG;
use embassy_rp::trng::Trng;

use crate::signals::{Message, SignerMessage, MESSAGE_QUEUE, SIGNER_CHANNEL};

#[embassy_executor::task]
pub async fn signer_task(mut trng: Trng<'static, TRNG>) -> ! {
    let mut seed: Option<Mnemonic> = None;
    loop {
        let msg = SIGNER_CHANNEL.receive().await;
        match msg {
            SignerMessage::GenerateSeed => {
                if seed.is_some() {
                    log_info!("Seed phrase already generated, skipping generation.");
                    continue;
                }
                let mut entropy = [0u8; 32];
                trng.fill_bytes(&mut entropy).await;
                let generated_seed = bip39::Mnemonic::from_entropy(&entropy)
                    .expect("Failed to generate seed phrase");
                seed = Some(generated_seed);
                let words = seed.as_ref().unwrap().words().collect::<Vec<_>>();
                MESSAGE_QUEUE
                    .send(Message::SeedPhraseGenerated(words.try_into().unwrap()))
                    .await;
            }
        }
    }
}
