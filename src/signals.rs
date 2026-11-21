use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU32};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;

pub(crate) use pico_ledger_core::signals::{Message, SignerMessage};

pub(crate) static MESSAGE_QUEUE: Channel<CriticalSectionRawMutex, Message, 4> = Channel::new();
pub(crate) static ACTIVITY_SIGNAL: Signal<CriticalSectionRawMutex, ()> = Signal::new();

pub(crate) static SIGNER_CHANNEL: Channel<CriticalSectionRawMutex, SignerMessage, 4> =
    Channel::new();

pub(crate) static DISPLAY_READY: AtomicBool = AtomicBool::new(false);
pub(crate) static DISPLAY_FRAME: AtomicU32 = AtomicU32::new(0);

pub(crate) struct UsbResponse {
    pub request_id: u16,
    pub result: Result<Vec<u8>, &'static str>,
}

pub(crate) static USB_RESPONSE_CHANNEL: Channel<CriticalSectionRawMutex, UsbResponse, 1> =
    Channel::new();
