use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;

pub(crate) enum Message {
    Next,
    SeedPhraseGenerated([&'static str; 24]),
    Back,
    Select,
    PasswordChecked(bool),
    Lock,
    TransactionSigned { signature: [u8; 64] },
}

pub(crate) static MESSAGE_QUEUE: Channel<CriticalSectionRawMutex, Message, 4> = Channel::new();
pub(crate) static ACTIVITY_SIGNAL: Signal<CriticalSectionRawMutex, ()> = Signal::new();

pub(crate) enum SignerMessage {
    GenerateSeed {
        password: [&'static str; 8],
    },
    CheckPassword {
        password: [&'static str; 8],
    },
    Lock,
    Sign {
        tx_hash: [u8; 32],
        address_index: u32,
    },
}

pub(crate) static SIGNER_CHANNEL: Channel<CriticalSectionRawMutex, SignerMessage, 1> =
    Channel::new();
