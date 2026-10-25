use buoyant::{match_view, view::prelude::*};
use embedded_graphics::{mono_font::ascii::FONT_6X10, pixelcolor::BinaryColor};

use crate::signals::{Message, SignerMessage, SIGNER_CHANNEL};

pub fn init_view() -> impl View<BinaryColor, AppModel> {
    VStack::new((
        Text::new("Bip39 Seed Generator", &FONT_6X10),
        Spacer::default(),
        Text::new("Press left to generate a new seed phrase", &FONT_6X10),
    ))
    .padding(Edges::Horizontal, 4)
    .padding(Edges::Vertical, 2)
}

pub fn home_view() -> impl View<BinaryColor, AppModel> {
    VStack::new((Text::new("Home Screen", &FONT_6X10), Spacer::default()))
        .padding(Edges::Horizontal, 4)
        .padding(Edges::Vertical, 2)
}

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

pub fn seed_phrase_view(
    w1: &'static str,
    w2: &'static str,
    w3: &'static str,
    page: usize,
) -> impl View<BinaryColor, AppModel> + use<> {
    VStack::new((
        Text::new("Seed Phrase", &FONT_6X10),
        Divider::new(1),
        Spacer::default(),
        Text::new(w1, &FONT_6X10),
        Text::new(w2, &FONT_6X10),
        Text::new(w3, &FONT_6X10),
        Spacer::default(),
        Text::new(PAGES_24[page], &FONT_6X10),
        Spacer::default(),
        Text::new("Press left to continue", &FONT_6X10),
    ))
}

pub enum AppModel {
    Init,
    Home,
    SeedPhrase {
        words: [&'static str; 24],
        step: usize,
    },
}

impl AppModel {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Next => {
                match self {
                    AppModel::Init => match SIGNER_CHANNEL.try_send(SignerMessage::GenerateSeed) {
                        Ok(_) => log_info!("Requested seed phrase generation."),
                        Err(_) => log_info!("Failed to request seed phrase generation."),
                    },
                    AppModel::Home => {
                        // Do nothing, as we're already on the home screen
                    }
                    AppModel::SeedPhrase { step, .. } => {
                        if *step < 7 {
                            *step += 1;
                        } else {
                            // Go back to Home after the last page
                            *self = AppModel::Home;
                        }
                    }
                }
            }
            Message::Back => {
                if let AppModel::SeedPhrase { step, .. } = self {
                    if *step > 0 {
                        *step -= 1;
                    }
                }
            }
            Message::SeedPhraseGenerated(words) => match self {
                AppModel::Init => {
                    *self = AppModel::SeedPhrase { words, step: 0 };
                }
                _ => {
                    log_info!("Received seed phrase while not on Home screen, ignoring.");
                }
            },
        }
    }

    pub fn view(&self) -> impl View<BinaryColor, AppModel> + use<> {
        match_view!(
            self,
            {
                AppModel::Init => init_view(),
                AppModel::Home => home_view(),
                AppModel::SeedPhrase { step, words } => {
                    let start = *step * 3;
                    let w1 = words[start];
                    let w2 = words[start + 1];
                    let w3 = words[start + 2];
                    seed_phrase_view(w1, w2, w3, *step)
                }
            }
        )
    }
}
