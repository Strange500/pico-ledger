use buoyant::view::{Divider, Spacer, Text, VStack, View};
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

pub(crate) struct SeedPhraseScreen {
    pub words: [&'static str; 24],
    pub step: usize,
}

impl SeedPhraseScreen {
    pub(crate) fn new(words: [&'static str; 24]) -> Self {
        Self { words, step: 0 }
    }

    pub(crate) fn view(&self) -> impl View<BinaryColor, AppModel> + use<> {
        let start = self.step * 3;

        let [w1, w2, w3] = match self.words.get(start..start + 3) {
            Some(&[a, b, c]) => [a, b, c],
            _ => ["", "", ""],
        };

        let page_text = PAGES_24.get(self.step).copied().unwrap_or("Seed Phrase");

        VStack::new((
            Text::new("Seed Phrase", &FONT_6X10),
            Divider::new(1),
            Spacer::default(),
            Text::new(w1, &FONT_6X10),
            Text::new(w2, &FONT_6X10),
            Text::new(w3, &FONT_6X10),
            Spacer::default(),
            Text::new(page_text, &FONT_6X10),
            Spacer::default(),
            Text::new("Press left to continue", &FONT_6X10),
        ))
    }

    pub(crate) fn update(&mut self, message: Message) -> Option<AppModel> {
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
