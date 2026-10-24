use core::cmp::{max, min};

use buoyant::{match_view, view::prelude::*};
use embedded_graphics::{mono_font::ascii::FONT_6X10, pixelcolor::BinaryColor};

use crate::signals::{Message, SEED_PHRASE};

pub fn home_view() -> impl View<BinaryColor, AppModel> {
    VStack::new((
        Text::new("Home", &FONT_6X10),
        Divider::new(1),
        Spacer::default(),
        Text::new("Welcome to Pico Ledger!", &FONT_6X10),
        Spacer::default(),
        Text::new("Press > to start", &FONT_6X10),
    ))
    .padding(Edges::Horizontal, 4)
    .padding(Edges::Vertical, 2)
}

pub fn seed_phrase_view(
    w1: &'static str,
    w2: &'static str,
    w3: &'static str,
) -> impl View<BinaryColor, AppModel> + use<> {
    VStack::new((
        Text::new("Seed Phrase", &FONT_6X10),
        Divider::new(1),
        Spacer::default(),
        Text::new(w1, &FONT_6X10),
        Text::new(w2, &FONT_6X10),
        Text::new(w3, &FONT_6X10),
        Spacer::default(),
        Text::new("Press > to continue", &FONT_6X10),
    ))
}

pub enum AppModel {
    Home,
    SeedPhrase {
        step: usize,
        words: [&'static str; 24],
    },
}

impl AppModel {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Next => {
                match self {
                    AppModel::Home => {
                        let seed_words = SEED_PHRASE
                            .lock(|seed| *seed.borrow())
                            .unwrap_or(["error"; 24]);
                        // Transition to SeedPhrase with example words
                        *self = AppModel::SeedPhrase {
                            step: 0,
                            words: seed_words,
                        };
                    }
                    AppModel::SeedPhrase { step, words } => {
                        if *step < (words.len() / 3) {
                            *step += 1;
                        } else {
                            // Reset to Home after last page
                            *self = AppModel::Home;
                        }
                    }
                }
            }
            Message::Back => {
                if let AppModel::SeedPhrase { step, .. } = self {
                    if *step > 0 {
                        *step -= 1;
                    } else {
                        // Go back to Home if on the first page
                        *self = AppModel::Home;
                    }
                }
            }
        }
    }

    pub fn view(&self) -> impl View<BinaryColor, AppModel> + use<> {
        match_view!(
            self,
            {
                AppModel::Home => home_view(),
                AppModel::SeedPhrase { step, words } => {
                    let start = *step * 3;
                    let w1 = words.get(start).unwrap_or(&"");
                    let w2 = words.get(start + 1).unwrap_or(&"");
                    let w3 = words.get(start + 2).unwrap_or(&"");
                    seed_phrase_view(w1, w2, w3)
                }
            }
        )
    }
}
