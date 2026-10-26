use core::time::Duration;

use buoyant::animation::Animation;
use buoyant::view::prelude::*;
use buoyant::view::{Spacer, Text, VStack, View};
use embedded_graphics::mono_font::ascii::FONT_6X10;
use embedded_graphics::pixelcolor::BinaryColor;

use crate::signals::Message;
use crate::ui::screens::seed_phrase_screen::SeedPhraseScreen;
use crate::ui::state::AppModel;

pub struct LoadingScreen;

impl LoadingScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn view(&self) -> impl View<BinaryColor, AppModel> {
        VStack::new((
            Text::new("Loading...", &FONT_6X10),
            Spacer::default(),
            Text::new(
                "Please wait while we generate your seed phrase.",
                &FONT_6X10,
            ),
        ))
        .animated(Animation::linear(Duration::from_secs(5)), true)
    }

    pub fn update(&mut self, message: Message) -> Option<AppModel> {
        match message {
            Message::SeedPhraseGenerated(words) => {
                Some(AppModel::SeedPhrase(SeedPhraseScreen::new(words)))
            }
            _ => None,
        }
    }
}
