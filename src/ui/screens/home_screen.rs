use buoyant::view::padding::Edges;
use buoyant::view::prelude::*;
use buoyant::view::{Spacer, Text, VStack, View};
use embedded_graphics::mono_font::ascii::FONT_6X10;
use embedded_graphics::pixelcolor::BinaryColor;

use crate::signals::{Message, SignerMessage, SIGNER_CHANNEL};
use crate::ui::state::AppModel;

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

    pub(crate) fn update(&mut self, message: Message) -> Option<AppModel> {
        if let Message::Next = message {
            let dummy_tx_hash = [1u8; 32];

            SIGNER_CHANNEL
                .try_send(SignerMessage::Sign {
                    tx_hash: dummy_tx_hash,
                    address_index: 1,
                })
                .ok();

            log_info!("Sent SignerMessage::Sign to SIGNER_CHANNEL");
            None
        } else if let Message::TransactionSigned { signature } = message {
            log::info!("Received signature: {signature:?}");
            None
        } else {
            None
        }
    }
}
