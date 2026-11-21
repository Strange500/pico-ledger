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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignerMessage {
    GenerateSeed { password: [&'static str; 8] },
    CheckPassword { password: [&'static str; 8] },
    Lock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandQueueFull;

/// Effects implemented by the firmware channel adapter or a host test double.
pub trait Effects {
    fn send(&mut self, command: SignerMessage) -> Result<(), CommandQueueFull>;
    fn activity(&mut self);
}
