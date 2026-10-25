use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;

pub enum Message {
    Next,
    SeedPhraseGenerated([&'static str; 24]),
    Back,
}

pub static MESSAGE_QUEUE: Channel<CriticalSectionRawMutex, Message, 4> = Channel::new();

pub enum SignerMessage {
    GenerateSeed,
}

pub static SIGNER_CHANNEL: Channel<CriticalSectionRawMutex, SignerMessage, 1> = Channel::new();
