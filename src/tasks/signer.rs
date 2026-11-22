use crate::signals::{
    Message, SignerMessage, UsbResponse, MESSAGE_QUEUE, SIGNER_CHANNEL, USB_RESPONSE_CHANNEL,
};
use embassy_rp::peripherals::TRNG;
use embassy_rp::trng::Trng;
use pico_ledger_core::wallet::{PreparedPsbt, Wallet};
#[cfg(feature = "test-mode")]
use pico_ledger_core::wallet::{TEST_MODE_ENTROPY, TEST_MODE_PASSWORD};

type PendingDescriptor = Option<(u16, alloc::string::String)>;

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

async fn begin_descriptor_export(
    request_id: u16,
    wallet: &Wallet,
    trng: &mut Trng<'static, TRNG>,
    psbt_pending: bool,
    pending: &mut PendingDescriptor,
) {
    if psbt_pending || pending.is_some() {
        USB_RESPONSE_CHANNEL
            .send(UsbResponse {
                request_id,
                result: Err("Another request is pending"),
            })
            .await;
        return;
    }
    let mut randomness = [0; 32];
    trng.fill_bytes(&mut randomness).await;
    match wallet.account_descriptor(&mut randomness) {
        Ok(descriptor) => {
            *pending = Some((request_id, descriptor));
            MESSAGE_QUEUE
                .send(Message::DescriptorReview { request_id })
                .await;
        }
        Err(error) => {
            USB_RESPONSE_CHANNEL
                .send(UsbResponse {
                    request_id,
                    result: Err(error),
                })
                .await;
        }
    }
}

async fn complete_descriptor_export(
    request_id: u16,
    approved: bool,
    pending: &mut PendingDescriptor,
) {
    if pending.as_ref().is_none_or(|(id, _)| *id != request_id) {
        return;
    }
    let Some((_, descriptor)) = pending.take() else {
        return;
    };
    let result = if approved {
        Ok(descriptor.into_bytes())
    } else {
        Err("Descriptor export rejected on device")
    };
    USB_RESPONSE_CHANNEL
        .send(UsbResponse { request_id, result })
        .await;
}

#[embassy_executor::task]
pub async fn signer_task(mut trng: Trng<'static, TRNG>) -> ! {
    #[cfg(feature = "test-mode")]
    log_warn!("UNSAFE TEST MODE: deterministic public wallet; never fund it");
    let mut wallet = Wallet::default();
    let mut pending = None;
    let mut pending_descriptor = None;
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
                #[cfg(feature = "test-mode")]
                let (mut entropy, password) = {
                    let _ = password;
                    (TEST_MODE_ENTROPY, TEST_MODE_PASSWORD)
                };
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
                if let Some((request_id, _)) = pending_descriptor.take() {
                    USB_RESPONSE_CHANNEL
                        .send(UsbResponse {
                            request_id,
                            result: Err("Device locked"),
                        })
                        .await;
                }
            }
            SignerMessage::CheckPassword { password } => {
                #[cfg(not(feature = "test-mode"))]
                let valid = wallet.unlock(&password).is_ok();
                #[cfg(feature = "test-mode")]
                let valid =
                    password == TEST_MODE_PASSWORD && wallet.unlock(&TEST_MODE_PASSWORD).is_ok();
                MESSAGE_QUEUE.send(Message::PasswordChecked(valid)).await;
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
            SignerMessage::ExportDescriptor { request_id } => {
                begin_descriptor_export(
                    request_id,
                    &wallet,
                    &mut trng,
                    pending.is_some(),
                    &mut pending_descriptor,
                )
                .await;
            }
            SignerMessage::ApproveDescriptor { request_id } => {
                complete_descriptor_export(request_id, true, &mut pending_descriptor).await;
            }
            SignerMessage::RejectDescriptor { request_id } => {
                complete_descriptor_export(request_id, false, &mut pending_descriptor).await;
            }
        }
    }
}
