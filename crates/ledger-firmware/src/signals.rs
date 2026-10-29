use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;

pub(crate) use ledger_core::{Message, SignerMessage};

pub(crate) static MESSAGE_QUEUE: Channel<CriticalSectionRawMutex, Message, 4> = Channel::new();
pub(crate) static ACTIVITY_SIGNAL: Signal<CriticalSectionRawMutex, ()> = Signal::new();

pub(crate) static SIGNER_CHANNEL: Channel<CriticalSectionRawMutex, SignerMessage, 1> =
    Channel::new();
