use buoyant::view::{Divider, ForEach, Spacer, Text, VStack, View};
use embedded_graphics::mono_font::ascii::FONT_6X10;
use embedded_graphics::mono_font::iso_8859_14::FONT_6X12;
use embedded_graphics::pixelcolor::BinaryColor;

use crate::ui::screens::loading_screen::{LoadingReason, LoadingScreen};
use crate::ui::state::{AppModel, Screen};
use crate::{Effect, Message, SignerMessage, Transition};

static DIGITS: [&str; 10] = ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"];

pub(crate) struct PasswordCreationScreen {
    pub password: [&'static str; 8],
    pub digit: u8,
}

impl PasswordCreationScreen {
    pub(crate) fn new() -> Self {
        Self {
            password: ["", "", "", "", "", "", "", ""],
            digit: 0,
        }
    }

    pub(crate) fn view(&self) -> impl View<BinaryColor, AppModel> {
        let digit = self.digit;
        VStack::new((
            Text::new("Create Password", &FONT_6X10),
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

    pub(crate) fn update(&mut self, message: Message) -> Transition {
        match message {
            Message::Next => {
                if self.password.iter().any(|&p| p.is_empty()) {
                    self.digit = if self.digit == 9 { 0 } else { self.digit + 1 };
                }
                Transition::none()
            }
            Message::Back => {
                if self.digit > 0 {
                    self.digit -= 1;
                } else {
                    self.digit = 9;
                }
                Transition::none()
            }
            Message::Select => {
                if self.password.iter().any(|&p| p.is_empty()) {
                    let index_empty = self.password.iter().position(|&p| p.is_empty());
                    if let Some(i) = index_empty {
                        if let Some(digit_str) = self.password.get_mut(i) {
                            if let Some(digit) = DIGITS.get(self.digit as usize) {
                                *digit_str = digit;
                            }
                        }
                    }
                }
                if self.password.iter().all(|&p| !p.is_empty()) {
                    Transition {
                        model: Some(AppModel {
                            screen: Screen::Loading(LoadingScreen::new(
                                LoadingReason::GeneratingSeedPhrase,
                            )),
                        }),
                        effect: Some(Effect::Signer(SignerMessage::GenerateSeed {
                            password: self.password,
                        })),
                    }
                } else {
                    Transition::none()
                }
            }
            _ => Transition::none(),
        }
    }
}
