use buoyant::{match_view, view::prelude::*};
use embedded_graphics::pixelcolor::BinaryColor;

use crate::signals::{Message, SignerMessage};
use crate::ui::screens::lockscreen::LockScreen;
use crate::ui::screens::{
    home_screen::HomeScreen, init_screen::InitScreen, loading_screen::LoadingScreen,
    password_creation_screen::PasswordCreationScreen, seed_phrase_screen::SeedPhraseScreen,
};

// Keep fixed-size state inline to avoid allocation in firmware message handling.
#[allow(clippy::large_enum_variant)]
pub enum AppModel {
    Init(InitScreen),
    PasswordCreation(PasswordCreationScreen),
    Loading(LoadingScreen),
    SeedPhrase(SeedPhraseScreen),
    Home(HomeScreen),
    Lock(LockScreen),
}

impl AppModel {
    pub fn update(&mut self, message: Message, effects: &mut impl crate::signals::Effects) {
        effects.activity();
        log::info!("Received message: ");
        if let Message::Lock = message {
            if !matches!(
                self,
                AppModel::Init(_)
                    | AppModel::PasswordCreation(_)
                    | AppModel::SeedPhrase(_)
                    | AppModel::Lock(_)
            ) {
                effects.send(SignerMessage::Lock).ok();
                *self = AppModel::Lock(LockScreen::new());
            }
            return;
        }

        let next = match self {
            AppModel::Init(screen) => screen.update(message, effects),
            AppModel::PasswordCreation(screen) => screen.update(message, effects),
            AppModel::Loading(screen) => screen.update(message, effects),
            AppModel::SeedPhrase(screen) => screen.update(message, effects),
            AppModel::Home(screen) => screen.update(message, effects),
            AppModel::Lock(screen) => screen.update(message, effects),
        };

        if let Some(new_model) = next {
            *self = new_model;
        }
    }

    pub fn view(&self) -> impl View<BinaryColor, AppModel> + use<> {
        match_view!(
            self,
            {
                AppModel::Init(screen) => screen.view(),
                AppModel::PasswordCreation(screen) => screen.view(),
                AppModel::Loading(screen) => screen.view(),
                AppModel::SeedPhrase(screen) => screen.view(),
                AppModel::Home(screen) => screen.view(),
                AppModel::Lock(screen) => screen.view(),
            }
        )
    }
}
