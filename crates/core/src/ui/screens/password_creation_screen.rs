use buoyant::animation::Animation;
use buoyant::view::prelude::*;
use buoyant::view::{ForEach, Text, VStack, View};
use core::time::Duration;
use embedded_graphics::mono_font::ascii::FONT_6X10;
use embedded_graphics::mono_font::iso_8859_14::FONT_6X12;
use embedded_graphics::pixelcolor::BinaryColor;

use crate::signals::{Message, SignerMessage};
use crate::ui::screens::loading_screen::{LoadingReason, LoadingScreen};
use crate::ui::state::AppModel;

static DIGITS: [&str; 10] = ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"];
static SELECTED_DIGITS: [&str; 10] = [
    "[0]", "[1]", "[2]", "[3]", "[4]", "[5]", "[6]", "[7]", "[8]", "[9]",
];
static PIN_STEPS: [&str; 8] = [
    "PIN 01/08",
    "PIN 02/08",
    "PIN 03/08",
    "PIN 04/08",
    "PIN 05/08",
    "PIN 06/08",
    "PIN 07/08",
    "PIN 08/08",
];

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
        let step = self
            .password
            .iter()
            .position(|&p| p.is_empty())
            .unwrap_or(7);
        let masked_password = self.password.map(|p| if p.is_empty() { "_" } else { "*" });
        VStack::new((
            Text::new("CREATE 8-DIGIT PIN", &FONT_6X10),
            Text::new(PIN_STEPS[step], &FONT_6X10),
            Rectangle
                .frame()
                .with_width(self.password.iter().filter(|p| !p.is_empty()).count() as u32 * 8)
                .with_height(3)
                .animated(
                    Animation::ease_out(Duration::from_millis(120)),
                    self.password.iter().filter(|p| !p.is_empty()).count() as u32,
                ),
            Text::new(masked_password.join(" "), &FONT_6X10),
            ForEach::<10>::new_horizontal(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9], move |i| {
                if *i == digit as usize {
                    Text::new(SELECTED_DIGITS[*i], &FONT_6X12)
                } else {
                    Text::new(DIGITS[*i], &FONT_6X10)
                }
            }),
            Text::new("R:+1 L:-1 BOTH:OK", &FONT_6X10),
        ))
    }

    pub fn update(
        &mut self,
        message: Message,
        effects: &mut impl crate::signals::Effects,
    ) -> Option<AppModel> {
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
                        if let Some(digit_str) = self.password.get_mut(i) {
                            if let Some(digit) = DIGITS.get(self.digit as usize) {
                                *digit_str = digit;
                            }
                        }
                    }
                }
                if self.password.iter().all(|&p| !p.is_empty()) {
                    if effects
                        .send(SignerMessage::GenerateSeed {
                            password: self.password,
                        })
                        .is_ok()
                    {
                        Some(AppModel::Loading(LoadingScreen::new(
                            LoadingReason::GeneratingSeedPhrase,
                        )))
                    } else {
                        log::info!("Failed to send GenerateSeed message");
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

impl Default for PasswordCreationScreen {
    fn default() -> Self {
        Self::new()
    }
}
