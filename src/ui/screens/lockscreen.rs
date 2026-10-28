use buoyant::view::prelude::*;
use buoyant::view::{Spacer, Text, VStack, View};
use embedded_graphics::mono_font::ascii::{FONT_6X10, FONT_6X12};
use embedded_graphics::pixelcolor::BinaryColor;

use crate::signals::{Message, SignerMessage, SIGNER_CHANNEL};
use crate::ui::screens::loading_screen::{LoadingReason, LoadingScreen};
use crate::ui::state::AppModel;
static DIGITS: [&str; 10] = ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"];

pub(crate) struct LockScreen {
    digit: u8,
    password: [&'static str; 8],
    has_error: bool,
}

impl LockScreen {
    pub(crate) fn new() -> Self {
        Self {
            digit: 0,
            password: ["", "", "", "", "", "", "", ""],
            has_error: false,
        }
    }

    pub(crate) fn with_error() -> Self {
        Self {
            digit: 0,
            password: ["", "", "", "", "", "", "", ""],
            has_error: true,
        }
    }

    pub(crate) fn view(&self) -> impl View<BinaryColor, AppModel> {
        let digit = self.digit;
        VStack::new((
            Text::new(
                if self.has_error {
                    "Invalid Password, try again"
                } else {
                    "Enter Password"
                },
                &FONT_6X10,
            ),
            Divider::new(1),
            Spacer::default(),
            Text::new(self.password.join(" "), &FONT_6X10),
            Spacer::default(),
            ForEach::<10>::new_horizontal(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9], move |i| {
                if *i == digit as usize {
                    Text::new(
                        match DIGITS.get(*i) {
                            Some(digit) => digit,
                            None => "",
                        },
                        &FONT_6X12,
                    )
                } else {
                    Text::new(
                        match DIGITS.get(*i) {
                            Some(digit) => digit,
                            None => "",
                        },
                        &FONT_6X10,
                    )
                }
            }),
        ))
    }

    pub(crate) fn update(&mut self, message: Message) -> Option<AppModel> {
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
                    match self.password.iter().position(|&p| p.is_empty()) {
                        Some(index) => {
                            if let Some(digit_str) = self.password.get_mut(index) {
                                if let Some(digit) = DIGITS.get(self.digit as usize) {
                                    *digit_str = digit;
                                }
                            }
                        }
                        None => {
                            log_info!("No empty password field found");
                        }
                    }
                }
                if self.password.iter().all(|&p| !p.is_empty()) {
                    if let Ok(()) = SIGNER_CHANNEL.try_send(SignerMessage::CheckPassword {
                        password: self.password,
                    }) {
                        Some(AppModel::Loading(LoadingScreen::new(
                            LoadingReason::CheckingPassword,
                        )))
                    } else {
                        log_error!("Failed to send password check message");
                        None
                    }
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}
