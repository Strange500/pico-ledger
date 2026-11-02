use buoyant::view::padding::Edges;
use buoyant::view::prelude::*;
use buoyant::view::{Spacer, Text, VStack, View};
use embedded_graphics::mono_font::ascii::FONT_6X10;
use embedded_graphics::pixelcolor::BinaryColor;

use crate::signals::{Message, SignerMessage};
use crate::ui::screens::lockscreen::LockScreen;
use crate::ui::state::AppModel;

pub struct HomeScreen;

impl HomeScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn view(&self) -> impl View<BinaryColor, AppModel> {
        VStack::new((
            Text::new("PICO LEDGER", &FONT_6X10),
            Text::new("Wallet unlocked", &FONT_6X10),
            Spacer::default(),
            Text::new("No pending request", &FONT_6X10),
            Text::new("RIGHT: LOCK WALLET", &FONT_6X10),
        ))
        .padding(Edges::Horizontal, 4)
        .padding(Edges::Vertical, 2)
    }

    pub fn update(
        &mut self,
        message: Message,
        effects: &mut impl crate::signals::Effects,
    ) -> Option<AppModel> {
        if let Message::Next = message {
            if effects.send(SignerMessage::Lock).is_ok() {
                Some(AppModel::Lock(LockScreen::new()))
            } else {
                log::error!("Failed to lock wallet");
                None
            }
        } else if let Message::TransactionSigned { signature } = message {
            log::info!("Received signature: {signature:?}");
            None
        } else {
            None
        }
    }
}

impl Default for HomeScreen {
    fn default() -> Self {
        Self::new()
    }
}
