use buoyant::view::padding::Edges;
use buoyant::view::prelude::*;
use buoyant::view::{Spacer, Text, VStack, View};
use embedded_graphics::mono_font::ascii::FONT_6X10;
use embedded_graphics::pixelcolor::BinaryColor;

use crate::signals::Message;
use crate::ui::screens::password_creation_screen::PasswordCreationScreen;
use crate::ui::state::AppModel;

pub struct InitScreen;

impl InitScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn view(&self) -> impl View<BinaryColor, AppModel> {
        VStack::new((
            Text::new("PICO LEDGER", &FONT_6X10),
            Text::new("Offline wallet", &FONT_6X10),
            Spacer::default(),
            Text::new("RIGHT: CREATE WALLET", &FONT_6X10),
        ))
        .padding(Edges::Horizontal, 4)
        .padding(Edges::Vertical, 2)
    }

    pub fn update(
        &mut self,
        message: Message,
        _effects: &mut impl crate::signals::Effects,
    ) -> Option<AppModel> {
        match message {
            Message::Next => Some(AppModel::PasswordCreation(PasswordCreationScreen::new())),
            _ => None,
        }
    }
}

impl Default for InitScreen {
    fn default() -> Self {
        Self::new()
    }
}
