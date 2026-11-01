#[derive(Debug, Clone, PartialEq, Eq)]
// Keep fixed-size state inline to avoid allocation in firmware message handling.
#[allow(clippy::large_enum_variant)]
pub enum Message {
    Next,
    SeedPhraseGenerated([&'static str; 24]),
    Back,
    Select,
    PasswordChecked(bool),
    Lock,
    TransactionSigned { signature: [u8; 64] },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignerMessage {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandQueueFull;

/// Effects implemented by the firmware channel adapter or a host test double.
pub trait Effects {
    fn send(&mut self, command: SignerMessage) -> Result<(), CommandQueueFull>;
    fn activity(&mut self);
}
