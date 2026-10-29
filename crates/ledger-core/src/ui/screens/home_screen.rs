use buoyant::view::padding::Edges;
use buoyant::view::prelude::*;
use buoyant::view::{Spacer, Text, VStack, View};
use embedded_graphics::mono_font::ascii::FONT_6X10;
use embedded_graphics::pixelcolor::BinaryColor;

use crate::ui::state::AppModel;
use crate::{Effect, Message, SignerMessage, Transition};

pub(crate) struct HomeScreen;

impl HomeScreen {
    pub(crate) fn new() -> Self {
        Self
    }

    pub(crate) fn view(&self) -> impl View<BinaryColor, AppModel> {
        VStack::new((Text::new("Home Screen", &FONT_6X10), Spacer::default()))
            .padding(Edges::Horizontal, 4)
            .padding(Edges::Vertical, 2)
    }

    pub(crate) fn update(&mut self, message: Message) -> Transition {
        if let Message::Next = message {
            let dummy_tx_hash = [1u8; 32];

            Transition::effect(Effect::Signer(SignerMessage::Sign {
                tx_hash: dummy_tx_hash,
                address_index: 1,
            }))
        } else if let Message::TransactionSigned { signature } = message {
            let _ = signature;
            Transition::none()
        } else {
            Transition::none()
        }
    }
}
