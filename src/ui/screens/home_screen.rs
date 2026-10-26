use buoyant::view::padding::Edges;
use buoyant::view::prelude::*;
use buoyant::view::{Spacer, Text, VStack, View};
use embedded_graphics::mono_font::ascii::FONT_6X10;
use embedded_graphics::pixelcolor::BinaryColor;

use crate::signals::{Message, SignerMessage, SIGNER_CHANNEL};
use crate::ui::state::AppModel;

pub struct HomeScreen;

impl HomeScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn view(&self) -> impl View<BinaryColor, AppModel> {
        VStack::new((Text::new("Home Screen", &FONT_6X10), Spacer::default()))
            .padding(Edges::Horizontal, 4)
            .padding(Edges::Vertical, 2)
    }

    pub fn update(&mut self, message: Message) -> Option<AppModel> {
        if let Message::Next = message {
            SIGNER_CHANNEL.try_send(SignerMessage::Test).ok();
            None
        } else {
            None
        }
    }
}
