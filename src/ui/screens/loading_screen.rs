use core::time::Duration;

use buoyant::animation::Animation;
use buoyant::view::prelude::*;
use buoyant::view::{Spacer, Text, VStack, View};
use embedded_graphics::mono_font::ascii::FONT_6X10;
use embedded_graphics::pixelcolor::BinaryColor;

use crate::signals::Message;
use crate::ui::screens::home_screen::HomeScreen;
use crate::ui::screens::lockscreen::LockScreen;
use crate::ui::screens::seed_phrase_screen::SeedPhraseScreen;
use crate::ui::state::AppModel;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LoadingReason {
    GeneratingSeedPhrase,
    CheckingPassword,
}

pub(crate) struct LoadingScreen {
    loading_reason: LoadingReason,
}

impl LoadingScreen {
    pub(crate) fn new(loading_reason: LoadingReason) -> Self {
        Self { loading_reason }
    }

    pub(crate) fn view(&self) -> impl View<BinaryColor, AppModel> {
        VStack::new((
            Text::new("Loading...", &FONT_6X10),
            Spacer::default(),
            Text::new(
                match self.loading_reason {
                    LoadingReason::GeneratingSeedPhrase => "Generating Seed Phrase...",
                    LoadingReason::CheckingPassword => "Checking Password...",
                },
                &FONT_6X10,
            ),
        ))
        .animated(Animation::linear(Duration::from_secs(5)), true)
    }

    pub(crate) fn update(&mut self, message: Message) -> Option<AppModel> {
        match message {
            Message::SeedPhraseGenerated(words) => {
                if self.loading_reason == LoadingReason::GeneratingSeedPhrase {
                    Some(AppModel::SeedPhrase(SeedPhraseScreen::new(words)))
                } else {
                    None
                }
            }
            Message::PasswordChecked(is_valid) => {
                if self.loading_reason == LoadingReason::CheckingPassword {
                    if is_valid {
                        Some(AppModel::Home(HomeScreen::new()))
                    } else {
                        Some(AppModel::Lock(LockScreen::with_error()))
                    }
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}
