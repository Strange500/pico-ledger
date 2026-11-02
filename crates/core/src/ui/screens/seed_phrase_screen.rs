use buoyant::animation::Animation;
use buoyant::view::prelude::*;
use buoyant::view::{Text, VStack, View};
use core::time::Duration;
use embedded_graphics::mono_font::ascii::FONT_6X10;
use embedded_graphics::pixelcolor::BinaryColor;

use crate::signals::Message;
use crate::ui::screens::home_screen::HomeScreen;
use crate::ui::state::AppModel;

static PAGES_24: [&str; 8] = [
    "Words 1-3 of 24",
    "Words 4-6 of 24",
    "Words 7-9 of 24",
    "Words 10-12 of 24",
    "Words 13-15 of 24",
    "Words 16-18 of 24",
    "Words 19-21 of 24",
    "Words 22-24 of 24",
];

pub struct SeedPhraseScreen {
    pub words: [&'static str; 24],
    pub step: usize,
}

impl SeedPhraseScreen {
    pub fn new(words: [&'static str; 24]) -> Self {
        Self { words, step: 0 }
    }

    pub fn view(&self) -> impl View<BinaryColor, AppModel> + use<> {
        let start = self.step * 3;

        let [w1, w2, w3] = match self.words.get(start..start + 3) {
            Some(&[a, b, c]) => [a, b, c],
            _ => ["", "", ""],
        };

        let page_text = PAGES_24.get(self.step).copied().unwrap_or("Seed Phrase");

        VStack::new((
            Text::new(page_text, &FONT_6X10),
            Rectangle
                .frame()
                .with_width((self.step as u32 + 1) * 8)
                .with_height(3)
                .animated(
                    Animation::ease_out(Duration::from_millis(120)),
                    self.step as u32,
                ),
            Text::new(w1, &FONT_6X10),
            Text::new(w2, &FONT_6X10),
            Text::new(w3, &FONT_6X10),
            Text::new("NEVER SHARE SEED", &FONT_6X10),
            Text::new("R:NEXT  L:BACK", &FONT_6X10),
        ))
    }

    pub fn update(
        &mut self,
        message: Message,
        _effects: &mut impl crate::signals::Effects,
    ) -> Option<AppModel> {
        match message {
            Message::Next => {
                if self.step < 7 {
                    self.step += 1;
                    None
                } else {
                    Some(AppModel::Home(HomeScreen::new()))
                }
            }
            Message::Back => {
                if self.step > 0 {
                    self.step -= 1;
                    None
                } else {
                    Some(AppModel::Home(HomeScreen::new()))
                }
            }
            _ => None,
        }
    }
}
