use crate::signals::{
    Message, SignerMessage, UsbResponse, MESSAGE_QUEUE, SIGNER_CHANNEL, USB_RESPONSE_CHANNEL,
};
use embassy_rp::peripherals::TRNG;
use embassy_rp::trng::Trng;
use pico_ledger_core::wallet::{PreparedPsbt, Wallet};

async fn review_psbt(
    request_id: u16,
    bytes: alloc::vec::Vec<u8>,
    wallet: &Wallet,
    trng: &mut Trng<'static, TRNG>,
    pending: &mut Option<(u16, PreparedPsbt)>,
) {
    if pending.is_some() {
        USB_RESPONSE_CHANNEL
            .send(UsbResponse {
                request_id,
                result: Err("Another transaction is pending"),
            })
            .await;
        return;
    }
    let mut randomness = [0; 32];
    trng.fill_bytes(&mut randomness).await;
    match wallet.prepare_psbt(&mut randomness, &bytes) {
        Ok(prepared) => {
            let review = prepared.review().clone();
            *pending = Some((request_id, prepared));
            MESSAGE_QUEUE
                .send(Message::PsbtReview { request_id, review })
                .await;
        }
        Err(error) => {
            USB_RESPONSE_CHANNEL
                .send(UsbResponse {
                    request_id,
                    result: Err(error),
                })
                .await;
            MESSAGE_QUEUE
                .send(Message::PsbtFailed { request_id, error })
                .await;
        }
    }
}

async fn approve_psbt(
    request_id: u16,
    wallet: &Wallet,
    trng: &mut Trng<'static, TRNG>,
    pending: &mut Option<(u16, PreparedPsbt)>,
) {
    let Some((pending_id, prepared)) = pending.take() else {
        return;
    };
    if pending_id != request_id {
        *pending = Some((pending_id, prepared));
        return;
    }
    let mut randomness = [0; 32];
    trng.fill_bytes(&mut randomness).await;
    let result = prepared.sign(wallet, &mut randomness);
    USB_RESPONSE_CHANNEL
        .send(UsbResponse { request_id, result })
        .await;
}

#[embassy_executor::task]
pub async fn signer_task(mut trng: Trng<'static, TRNG>) -> ! {
    let mut wallet = Wallet::default();
    let mut pending = None;
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
                            .await;
                    }
                    Err(err) => log_error!("Seed generation failed: {}", err),
                }
            }
            SignerMessage::Lock => {
                wallet.lock();
                if let Some((request_id, _)) = pending.take() {
                    USB_RESPONSE_CHANNEL
                        .send(UsbResponse {
                            request_id,
                            result: Err("Device locked"),
                        })
                        .await;
                }
            }
            SignerMessage::CheckPassword { password } => {
                MESSAGE_QUEUE
                    .send(Message::PasswordChecked(wallet.unlock(&password).is_ok()))
                    .await;
            }
            SignerMessage::ReviewPsbt { request_id, bytes } => {
                review_psbt(request_id, bytes, &wallet, &mut trng, &mut pending).await;
            }
            SignerMessage::ApprovePsbt { request_id } => {
                approve_psbt(request_id, &wallet, &mut trng, &mut pending).await;
            }
            SignerMessage::RejectPsbt { request_id } => {
                if pending.as_ref().is_some_and(|(id, _)| *id == request_id) {
                    pending = None;
                    USB_RESPONSE_CHANNEL
                        .send(UsbResponse {
                            request_id,
                            result: Err("Transaction rejected on device"),
                        })
                        .await;
                    MESSAGE_QUEUE
                        .send(Message::PsbtCancelled { request_id })
                        .await;
                }
            }
        }
    }
}
