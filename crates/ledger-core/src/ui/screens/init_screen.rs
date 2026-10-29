use buoyant::view::padding::Edges;
use buoyant::view::prelude::*;
use buoyant::view::{Spacer, Text, VStack, View};
use embedded_graphics::mono_font::ascii::FONT_6X10;
use embedded_graphics::pixelcolor::BinaryColor;

use crate::ui::screens::password_creation_screen::PasswordCreationScreen;
use crate::ui::state::{AppModel, Screen};
use crate::{Message, Transition};

pub(crate) struct InitScreen;

impl InitScreen {
    pub(crate) fn new() -> Self {
        Self
    }

    pub(crate) fn view(&self) -> impl View<BinaryColor, AppModel> {
        VStack::new((
            Text::new("Bip39 Seed Generator", &FONT_6X10),
            Spacer::default(),
            Text::new("Press left to generate a new seed phrase", &FONT_6X10),
        ))
        .padding(Edges::Horizontal, 4)
        .padding(Edges::Vertical, 2)
    }

    pub(crate) fn update(&mut self, message: Message) -> Transition {
        match message {
            Message::Next => Transition::model(AppModel {
                screen: Screen::PasswordCreation(PasswordCreationScreen::new()),
            }),
            _ => Transition::none(),
        }
    }
}
