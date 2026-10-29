#![no_std]

extern crate alloc;

pub mod ui;
pub use ui::AppModel;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Message {
    Next,
    SeedPhraseGenerated([&'static str; 24]),
    Back,
    Select,
    PasswordChecked(bool),
    Lock,
    TransactionSigned { signature: [u8; 64] },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
pub enum Effect {
    Signer(SignerMessage),
}

pub struct Transition {
    pub model: Option<ui::AppModel>,
    pub effect: Option<Effect>,
}

impl Transition {
    pub const fn none() -> Self {
        Self {
            model: None,
            effect: None,
        }
    }

    pub const fn model(model: ui::AppModel) -> Self {
        Self {
            model: Some(model),
            effect: None,
        }
    }

    pub const fn effect(effect: Effect) -> Self {
        Self {
            model: None,
            effect: Some(effect),
        }
    }
}
