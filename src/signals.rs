use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;

pub enum Message {
    Next,
    SeedPhraseGenerated([&'static str; 24]),
    Back,
    Select,
    PasswordChecked(bool),
    Lock,
}

pub static MESSAGE_QUEUE: Channel<CriticalSectionRawMutex, Message, 4> = Channel::new();
pub static ACTIVITY_SIGNAL: Signal<CriticalSectionRawMutex, ()> = Signal::new();

pub enum SignerMessage {
    GenerateSeed { password: [&'static str; 8] },
    CheckPassword { password: [&'static str; 8] },
    Lock,
    Test,
}

pub static SIGNER_CHANNEL: Channel<CriticalSectionRawMutex, SignerMessage, 1> = Channel::new();
