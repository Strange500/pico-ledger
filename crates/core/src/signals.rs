#[derive(Debug, Clone, PartialEq, Eq)]
// Keep fixed-size state inline to avoid allocation in firmware message handling.
#[allow(clippy::large_enum_variant)]
pub enum Message {
    Next,
    SeedPhraseGenerated([&'static str; 24]),
    Back,
    Select,
    #[cfg(feature = "psbt-demo")]
    PhysicalButton {
        button: PhysicalButton,
        frame: u32,
    },
    PasswordChecked(bool),
    Lock,
    #[cfg(feature = "psbt-demo")]
    DemoSigned {
        token: u64,
        ok: bool,
    },
    #[cfg(feature = "psbt-demo")]
    DemoPrepared {
        token: u64,
        review: Result<crate::psbt::TransactionReview, &'static str>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignerMessage {
    #[cfg(feature = "psbt-demo")]
    PrepareDemo,
    #[cfg(feature = "psbt-demo")]
    ConfirmDemo {
        token: u64,
    },
    #[cfg(feature = "psbt-demo")]
    CancelDemo {
        token: u64,
    },
    GenerateSeed {
        password: [&'static str; 8],
    },
    CheckPassword {
        password: [&'static str; 8],
    },
    Lock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandQueueFull;

#[cfg(feature = "psbt-demo")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhysicalButton {
    Left,
    Right,
    Both,
}

#[cfg(feature = "psbt-demo")]
impl Message {
    /// Discard physical presses begun before the currently visible frame.
    /// Zero means no rendered frame; MAX disables input on epoch exhaustion.
    pub fn for_display_frame(self, current: u32) -> Option<Self> {
        match self {
            Self::PhysicalButton { button, frame } => {
                if current == 0 || current == u32::MAX || frame != current {
                    return None;
                }
                Some(match button {
                    PhysicalButton::Left => Self::Back,
                    PhysicalButton::Right => Self::Next,
                    PhysicalButton::Both => Self::Select,
                })
            }
            other => Some(other),
        }
    }
}

/// Effects implemented by the firmware channel adapter or a host test double.
pub trait Effects {
    fn send(&mut self, command: SignerMessage) -> Result<(), CommandQueueFull>;
    fn activity(&mut self);
}
