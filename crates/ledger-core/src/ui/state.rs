use buoyant::{match_view, view::prelude::*};
use embedded_graphics::pixelcolor::BinaryColor;

use super::screens::lockscreen::LockScreen;
use super::screens::{
    home_screen::HomeScreen, init_screen::InitScreen, loading_screen::LoadingScreen,
    password_creation_screen::PasswordCreationScreen, seed_phrase_screen::SeedPhraseScreen,
};
use crate::{Effect, Message};

pub struct AppModel {
    pub(crate) screen: Screen,
}

pub(crate) enum Screen {
    Init(InitScreen),
    PasswordCreation(PasswordCreationScreen),
    Loading(LoadingScreen),
    SeedPhrase(SeedPhraseScreen),
    Home(HomeScreen),
    Lock(LockScreen),
}

impl AppModel {
    pub fn new() -> Self {
        Self {
            screen: Screen::Init(InitScreen::new()),
        }
    }

    pub fn update(&mut self, message: Message) -> Option<Effect> {
        if let Message::Lock = message {
            if !matches!(
                &self.screen,
                Screen::Init(_)
                    | Screen::PasswordCreation(_)
                    | Screen::SeedPhrase(_)
                    | Screen::Lock(_)
            ) {
                self.screen = Screen::Lock(LockScreen::new());
                return Some(Effect::Signer(crate::SignerMessage::Lock));
            }
            return None;
        }

        let transition = match &mut self.screen {
            Screen::Init(screen) => screen.update(message),
            Screen::PasswordCreation(screen) => screen.update(message),
            Screen::Loading(screen) => screen.update(message),
            Screen::SeedPhrase(screen) => screen.update(message),
            Screen::Home(screen) => screen.update(message),
            Screen::Lock(screen) => screen.update(message),
        };

        if let Some(model) = transition.model {
            self.screen = model.screen;
        }
        transition.effect
    }

    pub fn view(&self) -> impl View<BinaryColor, AppModel> + use<> {
        match_view!(
            &self.screen,
            {
                Screen::Init(screen) => screen.view(),
                Screen::PasswordCreation(screen) => screen.view(),
                Screen::Loading(screen) => screen.view(),
                Screen::SeedPhrase(screen) => screen.view(),
                Screen::Home(screen) => screen.view(),
                Screen::Lock(screen) => screen.view(),
            }
        )
    }
}

impl Default for AppModel {
    fn default() -> Self {
        Self::new()
    }
}
