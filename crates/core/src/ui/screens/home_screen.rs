use buoyant::view::padding::Edges;
use buoyant::view::prelude::*;
use buoyant::view::{Spacer, Text, VStack, View};
use embedded_graphics::mono_font::ascii::FONT_6X10;
use embedded_graphics::pixelcolor::BinaryColor;

use crate::signals::Message;
use crate::ui::state::AppModel;

pub struct HomeScreen {
    #[cfg(feature = "psbt-demo")]
    busy: bool,
    status: &'static str,
}

impl HomeScreen {
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "psbt-demo")]
            busy: false,
            status: if cfg!(feature = "psbt-demo") {
                "Left: sign demo"
            } else {
                "Signing disabled"
            },
        }
    }

    pub fn view(&self) -> impl View<BinaryColor, AppModel> {
        VStack::new((
            Text::new(
                if cfg!(feature = "psbt-demo") {
                    "TEST ONLY - PSBT"
                } else {
                    "Home Screen"
                },
                &FONT_6X10,
            ),
            Text::new(
                if cfg!(feature = "psbt-demo") {
                    "PUBLIC KEY / NO FUNDS"
                } else {
                    ""
                },
                &FONT_6X10,
            ),
            Text::new(self.status, &FONT_6X10),
            Spacer::default(),
        ))
        .padding(Edges::Horizontal, 4)
        .padding(Edges::Vertical, 2)
    }

    pub fn update(
        &mut self,
        _message: Message,
        _effects: &mut impl crate::signals::Effects,
    ) -> Option<AppModel> {
        // Production signing remains disconnected. Back is the debounced physical
        // left button; only this screen requests the closed public-key fixture.
        #[cfg(feature = "psbt-demo")]
        match _message {
            Message::Back if !self.busy => {
                if _effects
                    .send(crate::signals::SignerMessage::SignDemo)
                    .is_ok()
                {
                    self.busy = true;
                    self.status = "Signing demo...";
                } else {
                    self.status = "Busy - retry left";
                }
            }
            Message::DemoSigned(ok) if self.busy => {
                self.busy = false;
                self.status = if ok {
                    "Signed! USB hex"
                } else {
                    "Failed - retry left"
                };
            }
            _ => {}
        }
        None
    }
}

impl Default for HomeScreen {
    fn default() -> Self {
        Self::new()
    }
}
