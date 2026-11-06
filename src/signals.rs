use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;

pub(crate) use pico_ledger_core::signals::{Message, SignerMessage};

pub(crate) static MESSAGE_QUEUE: Channel<CriticalSectionRawMutex, Message, 4> = Channel::new();
pub(crate) static ACTIVITY_SIGNAL: Signal<CriticalSectionRawMutex, ()> = Signal::new();

// Only the display task writes this after a successfully flushed frame.
#[cfg(feature = "psbt-demo")]
pub(crate) static DISPLAY_FRAME: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

pub(crate) static SIGNER_CHANNEL: Channel<CriticalSectionRawMutex, SignerMessage, 1> =
    Channel::new();
