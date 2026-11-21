use alloc::vec;
use core::sync::atomic::Ordering;
use embassy_futures::join::join3;
use embassy_rp::{peripherals::USB, usb::Driver};
use embassy_usb::{
    class::cdc_acm::{CdcAcmClass, State},
    driver::Driver as UsbDriver,
    Builder, Config,
};
use embedded_io_async::{Read, Write};
use pico_ledger_core::{
    psbt::MAX_PSBT_BYTES,
    signals::SignerMessage,
    transport::{Header, Kind, HEADER_LEN},
};
use static_cell::StaticCell;

use crate::signals::{DISPLAY_READY, MESSAGE_QUEUE, SIGNER_CHANNEL, USB_RESPONSE_CHANNEL};

const MAX_PACKET_SIZE: u16 = 64;

struct UsbResources {
    logger_state: State<'static>,
    protocol_state: State<'static>,
    config_descriptor: [u8; 256],
    bos_descriptor: [u8; 32],
    msos_descriptor: [u8; 16],
    control_buf: [u8; 64],
}

impl UsbResources {
    const fn new() -> Self {
        Self {
            logger_state: State::new(),
            protocol_state: State::new(),
            config_descriptor: [0; 256],
            bos_descriptor: [0; 32],
            msos_descriptor: [0; 16],
            control_buf: [0; 64],
        }
    }
}

static USB_RESOURCES: StaticCell<UsbResources> = StaticCell::new();
static PROTOCOL_RX_BUFFER: StaticCell<[u8; MAX_PACKET_SIZE as usize]> = StaticCell::new();

/// Runs a composite USB device with one logging port and one PSBT protocol port.
#[embassy_executor::task]
pub async fn usb_task(driver: Driver<'static, USB>) {
    let resources = USB_RESOURCES.init(UsbResources::new());
    let mut config = Config::new(0xc0de, 0xcafe);
    config.manufacturer = Some("pico-ledger");
    config.product = Some("pico-ledger");
    config.serial_number = Some("PICOLEDGER1");

    let mut builder = Builder::new(
        driver,
        config,
        &mut resources.config_descriptor,
        &mut resources.bos_descriptor,
        &mut resources.msos_descriptor,
        &mut resources.control_buf,
    );
    let logger_class = CdcAcmClass::new(&mut builder, &mut resources.logger_state, MAX_PACKET_SIZE);
    let protocol_class =
        CdcAcmClass::new(&mut builder, &mut resources.protocol_state, MAX_PACKET_SIZE);
    let mut device = builder.build();

    let logger = embassy_usb_logger::with_class!(1024, log::LevelFilter::Info, logger_class);
    let protocol = protocol_loop(protocol_class);
    join3(device.run(), logger, protocol).await;
}

async fn protocol_loop<'d, D>(class: CdcAcmClass<'d, D>) -> !
where
    D: UsbDriver<'d>,
{
    let (mut sender, receiver) = class.split();
    let mut receiver = receiver.into_buffered(PROTOCOL_RX_BUFFER.init([0; 64]));

    loop {
        sender.wait_connection().await;
        let mut encoded_header = [0; HEADER_LEN];
        if receiver.read_exact(&mut encoded_header).await.is_err() {
            continue;
        }
        let request_id = u16::from_le_bytes([encoded_header[6], encoded_header[7]]);
        let header = match Header::decode(&encoded_header) {
            Ok(header) => header,
            Err(error) => {
                if write_response(&mut sender, request_id, Kind::Error, error.as_bytes())
                    .await
                    .is_err()
                {
                    continue;
                }
                continue;
            }
        };

        match header.kind {
            Kind::Ping if header.payload_len == 0 => {
                let _ = write_response(&mut sender, header.request_id, Kind::Pong, &[]).await;
            }
            Kind::SignPsbt => {
                let Ok(payload_len) = usize::try_from(header.payload_len) else {
                    let _ = write_response(
                        &mut sender,
                        header.request_id,
                        Kind::Error,
                        b"Invalid payload length",
                    )
                    .await;
                    continue;
                };
                if payload_len == 0 || payload_len > MAX_PSBT_BYTES {
                    let _ = write_response(
                        &mut sender,
                        header.request_id,
                        Kind::Error,
                        b"PSBT payload exceeds limit",
                    )
                    .await;
                    continue;
                }
                if write_response(&mut sender, header.request_id, Kind::Ready, &[])
                    .await
                    .is_err()
                {
                    continue;
                }
                let mut bytes = vec![0; payload_len];
                if receiver.read_exact(&mut bytes).await.is_err() {
                    continue;
                }
                sign_and_respond(&mut sender, header.request_id, bytes).await;
            }
            Kind::ExportDescriptor if header.payload_len == 0 => {
                export_descriptor(&mut sender, header.request_id).await;
            }
            _ => {
                let _ = write_response(
                    &mut sender,
                    header.request_id,
                    Kind::Error,
                    b"Unsupported request",
                )
                .await;
            }
        }
    }
}

async fn export_descriptor<'d, D>(
    sender: &mut embassy_usb::class::cdc_acm::Sender<'d, D>,
    request_id: u16,
) where
    D: UsbDriver<'d>,
{
    if !DISPLAY_READY.load(Ordering::Acquire) {
        let _ = write_response(sender, request_id, Kind::Error, b"Display unavailable").await;
        return;
    }
    SIGNER_CHANNEL
        .send(SignerMessage::ExportDescriptor { request_id })
        .await;
    let response = USB_RESPONSE_CHANNEL.receive().await;
    if response.request_id != request_id {
        let _ = write_response(
            sender,
            request_id,
            Kind::Error,
            b"Internal request mismatch",
        )
        .await;
        return;
    }
    match response.result {
        Ok(descriptor) => {
            if write_response(sender, request_id, Kind::Descriptor, &descriptor)
                .await
                .is_ok()
            {
                MESSAGE_QUEUE
                    .send(pico_ledger_core::signals::Message::DescriptorExported)
                    .await;
            }
        }
        Err(error) => {
            let _ = write_response(sender, request_id, Kind::Error, error.as_bytes()).await;
        }
    }
}

async fn sign_and_respond<'d, D>(
    sender: &mut embassy_usb::class::cdc_acm::Sender<'d, D>,
    request_id: u16,
    bytes: alloc::vec::Vec<u8>,
) where
    D: UsbDriver<'d>,
{
    if !DISPLAY_READY.load(Ordering::Acquire) {
        let _ = write_response(sender, request_id, Kind::Error, b"Display unavailable").await;
        return;
    }
    SIGNER_CHANNEL
        .send(SignerMessage::ReviewPsbt { request_id, bytes })
        .await;
    let response = USB_RESPONSE_CHANNEL.receive().await;
    if response.request_id != request_id {
        let _ = write_response(
            sender,
            request_id,
            Kind::Error,
            b"Internal request mismatch",
        )
        .await;
        return;
    }
    match response.result {
        Ok(bytes) => {
            if write_response(sender, request_id, Kind::SignedPsbt, &bytes)
                .await
                .is_ok()
            {
                MESSAGE_QUEUE
                    .send(pico_ledger_core::signals::Message::PsbtSigned { request_id })
                    .await;
            } else {
                MESSAGE_QUEUE
                    .send(pico_ledger_core::signals::Message::PsbtFailed {
                        request_id,
                        error: "USB disconnected",
                    })
                    .await;
            }
        }
        Err(error) => {
            let _ = write_response(sender, request_id, Kind::Error, error.as_bytes()).await;
        }
    }
}

async fn write_response<'d, D>(
    sender: &mut embassy_usb::class::cdc_acm::Sender<'d, D>,
    request_id: u16,
    kind: Kind,
    payload: &[u8],
) -> Result<(), embassy_usb::class::cdc_acm::CdcAcmError>
where
    D: UsbDriver<'d>,
{
    let payload_len = payload.len() as u32;
    sender
        .write_all(
            &Header {
                kind,
                request_id,
                payload_len,
            }
            .encode(),
        )
        .await?;
    sender.write_all(payload).await
}
