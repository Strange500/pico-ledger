use buoyant::view::{Divider, ForEach, Spacer, Text, VStack, View};
use embedded_graphics::mono_font::ascii::FONT_6X10;
use embedded_graphics::mono_font::iso_8859_14::FONT_6X12;
use embedded_graphics::pixelcolor::BinaryColor;

use crate::signals::{Message, SignerMessage, SIGNER_CHANNEL};
use crate::ui::screens::loading_screen::{LoadingReason, LoadingScreen};
use crate::ui::state::AppModel;

static DIGITS: [&str; 10] = ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"];

pub struct PasswordCreationScreen {
    pub password: [&'static str; 8],
    pub digit: u8,
}

impl PasswordCreationScreen {
    pub fn new() -> Self {
        Self {
            password: ["", "", "", "", "", "", "", ""],
            digit: 0,
        }
    }

    pub fn view(&self) -> impl View<BinaryColor, AppModel> {
        let digit = self.digit;
        VStack::new((
            Text::new("Create Password", &FONT_6X10),
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
                    let index_empty = self.password.iter().position(|&p| p.is_empty());
                    if let Some(i) = index_empty {
                        self.password[i] = DIGITS[self.digit as usize];
                    }
                }
                if self.password.iter().all(|&p| !p.is_empty()) {
                    match SIGNER_CHANNEL.try_send(SignerMessage::GenerateSeed {
                        password: self.password,
                    }) {
                        Ok(_) => Some(AppModel::Loading(LoadingScreen::new(
                            LoadingReason::GeneratingSeedPhrase,
                        ))),
                        Err(_) => {
                            log_info!("Failed to send GenerateSeed message");
                            None
                        }
                    }
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}
