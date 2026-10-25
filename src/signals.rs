use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;

pub enum Message {
    Next,
    SeedPhraseGenerated([&'static str; 24]),
    Back,
    Select,
}

pub static MESSAGE_QUEUE: Channel<CriticalSectionRawMutex, Message, 4> = Channel::new();

pub enum SignerMessage {
    GenerateSeed { password: [&'static str; 8] },
    TestSeed { password: [&'static str; 8] },
}

pub static SIGNER_CHANNEL: Channel<CriticalSectionRawMutex, SignerMessage, 1> = Channel::new();
