use alloc::vec::Vec;

use crate::psbt::TransactionReview;

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
    PsbtReview {
        request_id: u16,
        review: TransactionReview,
    },
    PsbtFailed {
        request_id: u16,
        error: &'static str,
    },
    PsbtSigned {
        request_id: u16,
    },
    PsbtCancelled {
        request_id: u16,
    },
    PhysicalButton {
        button: PhysicalButton,
        frame: u32,
    },
    DescriptorReview {
        request_id: u16,
    },
    DescriptorExported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhysicalButton {
    Left,
    Right,
    Both,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignerMessage {
    GenerateSeed { password: [&'static str; 8] },
    CheckPassword { password: [&'static str; 8] },
    Lock,
    ReviewPsbt { request_id: u16, bytes: Vec<u8> },
    ApprovePsbt { request_id: u16 },
    RejectPsbt { request_id: u16 },
    ExportDescriptor { request_id: u16 },
    ApproveDescriptor { request_id: u16 },
    RejectDescriptor { request_id: u16 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandQueueFull;

/// Effects implemented by the firmware channel adapter or a host test double.
pub trait Effects {
    fn send(&mut self, command: SignerMessage) -> Result<(), CommandQueueFull>;
    fn activity(&mut self);
}
