use crate::signals::{Message, SignerMessage, MESSAGE_QUEUE, SIGNER_CHANNEL};
use embassy_rp::peripherals::TRNG;
use embassy_rp::trng::Trng;
use pico_ledger_core::wallet::Wallet;

#[embassy_executor::task]
pub async fn signer_task(mut trng: Trng<'static, TRNG>) -> ! {
    let mut wallet = Wallet::default();
    #[cfg(feature = "psbt-demo")]
    let mut demo = pico_ledger_core::demo::DemoSession::default();
    loop {
        match SIGNER_CHANNEL.receive().await {
            #[cfg(feature = "psbt-demo")]
            SignerMessage::PrepareDemo => {
                let mut randomness = [0; 32];
                trng.fill_bytes(&mut randomness).await;
                let (token, review) = match demo.prepare(&mut randomness) {
                    Ok((token, review)) => (token, Ok(review)),
                    Err(error) => (0, Err(error)),
                };
                MESSAGE_QUEUE
                    .send(Message::DemoPrepared { token, review })
                    .await;
            }
            #[cfg(feature = "psbt-demo")]
            SignerMessage::CancelDemo { token } => {
                demo.cancel(token);
            }
            #[cfg(feature = "psbt-demo")]
            SignerMessage::ConfirmDemo { token } => {
                // Mismatched, duplicate, cancelled or pre-review confirms cannot sign.
                let Some(prepared) = demo.confirm(token) else {
                    continue;
                };
                // Never pass the user's wallet to this closed public-key demo.
                let mut randomness = [0; 32];
                trng.fill_bytes(&mut randomness).await;
                let result = prepared.sign(&mut randomness);
                match &result {
                    Ok(bytes) => {
                        use core::fmt::Write;
                        let mut hex = alloc::string::String::with_capacity(bytes.len() * 2);
                        for byte in bytes {
                            let _ = write!(hex, "{byte:02x}");
                        }
                        log_info!("TEST ONLY signed PSBT hex: {}", hex.as_str());
                    }
                    Err(err) => log_error!("TEST ONLY PSBT failed: {}", err),
                }
                MESSAGE_QUEUE
                    .send(Message::DemoSigned {
                        token,
                        ok: result.is_ok(),
                    })
                    .await;
            }
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
                            .await;
                    }
                    Err(err) => log_error!("Seed generation failed: {}", err),
                }
            }
            SignerMessage::Lock => {
                #[cfg(feature = "psbt-demo")]
                {
                    demo.lock();
                }
                wallet.lock();
            }
            SignerMessage::CheckPassword { password } => {
                MESSAGE_QUEUE
                    .send(Message::PasswordChecked(wallet.unlock(&password).is_ok()))
                    .await;
            }
        }
    }
}
