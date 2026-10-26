use buoyant::view::prelude::*;
use buoyant::view::{Spacer, Text, VStack, View};
use embedded_graphics::mono_font::ascii::{FONT_6X10, FONT_6X12};
use embedded_graphics::pixelcolor::BinaryColor;

use crate::signals::{Message, SignerMessage, SIGNER_CHANNEL};
use crate::ui::screens::loading_screen::{LoadingReason, LoadingScreen};
use crate::ui::state::AppModel;
static DIGITS: [&str; 10] = ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"];

pub struct LockScreen {
    digit: u8,
    password: [&'static str; 8],
    has_error: bool,
}

impl LockScreen {
    pub fn new() -> Self {
        Self {
            digit: 0,
            password: ["", "", "", "", "", "", "", ""],
            has_error: false,
        }
    }

    pub fn with_error() -> Self {
        Self {
            digit: 0,
            password: ["", "", "", "", "", "", "", ""],
            has_error: true,
        }
    }

    pub fn view(&self) -> impl View<BinaryColor, AppModel> {
        let digit = self.digit;
        VStack::new((
            Text::new(
                match self.has_error {
                    true => "Invalid Password, try again",
                    false => "Enter Password",
                },
                &FONT_6X10,
            ),
            Divider::new(1),
            Spacer::default(),
            Text::new(self.password.join(" "), &FONT_6X10),
            Spacer::default(),
            ForEach::<10>::new_horizontal(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9], move |i| {
                if *i == digit as usize {
                    Text::new(DIGITS[*i], &FONT_6X12)
                } else {
                    Text::new(DIGITS[*i], &FONT_6X10)
                }
            }),
        ))
    }

    pub fn update(&mut self, message: Message) -> Option<AppModel> {
        match message {
            Message::Next => {
                if self.password.iter().any(|&p| p.is_empty()) {
                    self.digit = if self.digit == 9 { 0 } else { self.digit + 1 };
                }
                None
            }
            Message::Back => {
                if self.digit > 0 {
                    self.digit -= 1;
                } else {
                    self.digit = 9;
                }
                None
            }
            Message::Select => {
                if self.password.iter().any(|&p| p.is_empty()) {
                    let index_empty = self.password.iter().position(|&p| p.is_empty()).unwrap();
                    self.password[index_empty] = DIGITS[self.digit as usize];
                }
                if self.password.iter().all(|&p| !p.is_empty()) {
                    SIGNER_CHANNEL
                        .try_send(SignerMessage::CheckPassword {
                            password: self.password,
                        })
                        .unwrap_or_else(|_| {
                            log_info!("Failed to send checkPassword message to signer")
                        });
                    Some(AppModel::Loading(LoadingScreen::new(
                        LoadingReason::CheckingPassword,
                    )))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}
