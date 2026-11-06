#[cfg(feature = "psbt-demo")]
use crate::{psbt::TransactionReview, signals::SignerMessage};
use crate::{signals::Message, ui::state::AppModel};
#[cfg(feature = "psbt-demo")]
use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};
use buoyant::view::padding::Edges;
use buoyant::view::prelude::*;
use buoyant::view::{Spacer, Text, VStack, View};
use embedded_graphics::mono_font::ascii::FONT_6X10;
use embedded_graphics::pixelcolor::BinaryColor;

#[cfg(feature = "psbt-demo")]
#[derive(Default)]
enum Phase {
    #[default]
    Idle,
    Preparing,
    Reviewing,
    Decision,
    Signing,
}

#[cfg(feature = "psbt-demo")]
pub struct ReviewPage {
    pub title: String,
    pub body: String,
}

pub struct HomeScreen {
    status: &'static str,
    #[cfg(feature = "psbt-demo")]
    phase: Phase,
    #[cfg(feature = "psbt-demo")]
    pages: Vec<ReviewPage>,
    #[cfg(feature = "psbt-demo")]
    page: usize,
    #[cfg(feature = "psbt-demo")]
    token: u64,
    #[cfg(feature = "psbt-demo")]
    sign_selected: bool,
    #[cfg(feature = "psbt-demo")]
    visible: bool,
}

#[cfg(feature = "psbt-demo")]
fn pages(review: &TransactionReview) -> Vec<ReviewPage> {
    let mut pages = Vec::new();
    // All policy data is ASCII. Exactly 20 columns and at most 3 value lines
    // on the 128x64 OLED. Long identifiers are split, NEVER truncated/ellipsized.
    let mut field = |title: String, value: String| {
        let chunks: Vec<_> = value.as_bytes().chunks(60).collect();
        for (part, chunk) in chunks.iter().enumerate() {
            let mut body = String::new();
            for (line, text) in chunk.chunks(20).enumerate() {
                if line != 0 {
                    body.push('\n');
                }
                body.push_str(core::str::from_utf8(text).unwrap_or("INVALID ASCII"));
            }
            pages.push(ReviewPage {
                title: if chunks.len() > 1 {
                    format!("{title} {}/{}", part + 1, chunks.len())
                } else {
                    title.clone()
                },
                body,
            });
        }
    };
    field("PUBLIC KEY".into(), "TESTNET / NO FUNDS".into());
    field("Transaction ID".into(), review.txid.to_string());
    field("Version".into(), review.version.to_string());
    field("Lock time".into(), review.lock_time.to_string());
    field(
        "Inputs / Outputs".into(),
        format!("{} / {}", review.input_count, review.outputs.len()),
    );
    for (i, input) in review.inputs.iter().enumerate() {
        field(format!("In{} outpoint", i + 1), input.outpoint.to_string());
        field(format!("In{} address", i + 1), input.address.clone());
        field(
            format!("In{} sats (owned)", i + 1),
            input.amount_sat.to_string(),
        );
        field(
            format!("In{} sequence", i + 1),
            review.sequences[i].to_string(),
        );
    }
    for (i, output) in review.outputs.iter().enumerate() {
        field(format!("Out{} address", i + 1), output.address.clone());
        field(format!("Out{} sats", i + 1), output.amount_sat.to_string());
        field(
            format!("Out{} type", i + 1),
            if output.is_change {
                "Verified change"
            } else {
                "External recipient"
            }
            .into(),
        );
    }
    field(
        "Total input sats".into(),
        review.total_input_sat.to_string(),
    );
    field(
        "Total output sats".into(),
        review.total_output_sat.to_string(),
    );
    field("Fee sats".into(), review.fee_sat.to_string());
    field("Sighash".into(), "ALL (all inputs)".into());
    pages
}

impl HomeScreen {
    pub fn new() -> Self {
        Self {
            status: if cfg!(feature = "psbt-demo") {
                "Left: review demo"
            } else {
                "Signing disabled"
            },
            #[cfg(feature = "psbt-demo")]
            phase: Phase::Idle,
            #[cfg(feature = "psbt-demo")]
            pages: Vec::new(),
            #[cfg(feature = "psbt-demo")]
            page: 0,
            #[cfg(feature = "psbt-demo")]
            token: 0,
            #[cfg(feature = "psbt-demo")]
            sign_selected: false,
            #[cfg(feature = "psbt-demo")]
            visible: false,
        }
    }
    #[cfg(feature = "psbt-demo")]
    pub fn review_pages(&self) -> &[ReviewPage] {
        &self.pages
    }

    /// Called by the display adapter ONLY after a successful OLED flush.
    #[cfg(feature = "psbt-demo")]
    pub fn displayed(&mut self) {
        self.visible = true;
    }

    pub fn view(&self) -> impl View<BinaryColor, AppModel> {
        #[cfg(feature = "psbt-demo")]
        let (header, title, body, nav) = match self.phase {
            Phase::Reviewing => {
                let p = &self.pages[self.page];
                (
                    format!("{}/{} TEST ONLY", self.page + 1, self.pages.len()),
                    p.title.clone(),
                    p.body.clone(),
                    "L:prev R:next B:exit",
                )
            }
            Phase::Decision => (
                "TEST ONLY".into(),
                "Confirm signing?".into(),
                if self.sign_selected {
                    "Cancel   >SIGN"
                } else {
                    ">CANCEL   Sign"
                }
                .into(),
                "L/R:choice B:select",
            ),
            _ => (
                "TEST ONLY - PSBT".into(),
                "PUBLIC KEY / NO FUNDS".into(),
                self.status.into(),
                "",
            ),
        };
        #[cfg(not(feature = "psbt-demo"))]
        let (header, title, body, nav) = ("Home Screen", "", self.status, "");
        VStack::new((
            Text::new(header, &FONT_6X10),
            Text::new(title, &FONT_6X10),
            Text::new(body, &FONT_6X10),
            Text::new(nav, &FONT_6X10),
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
        #[cfg(feature = "psbt-demo")]
        {
            if matches!(self.phase, Phase::Reviewing | Phase::Decision)
                && matches!(_message, Message::Next | Message::Back | Message::Select)
            {
                if !self.visible {
                    return None;
                }
                self.visible = false;
            }
            match (&self.phase, _message) {
                (Phase::Idle, Message::Back) => {
                    if _effects.send(SignerMessage::PrepareDemo).is_ok() {
                        self.phase = Phase::Preparing;
                        self.status = "Validating demo...";
                    } else {
                        self.status = "Busy - retry left";
                    }
                }
                (Phase::Preparing, Message::DemoPrepared { token, review }) => match review {
                    Ok(review) => {
                        self.pages = pages(&review);
                        self.page = 0;
                        self.token = token;
                        self.phase = Phase::Reviewing;
                        self.sign_selected = false;
                        self.visible = false;
                    }
                    Err(_) => {
                        self.phase = Phase::Idle;
                        self.status = "Invalid demo PSBT";
                    }
                },
                (Phase::Reviewing, Message::Next) => {
                    if self.page + 1 < self.pages.len() {
                        self.page += 1;
                    } else {
                        self.phase = Phase::Decision;
                        self.sign_selected = false;
                    }
                }
                (Phase::Reviewing, Message::Back) => {
                    self.page = self.page.saturating_sub(1);
                }
                (Phase::Reviewing, Message::Select) => {
                    self.cancel(_effects);
                }
                (Phase::Decision, Message::Back) => {
                    self.sign_selected = false;
                }
                (Phase::Decision, Message::Next) => {
                    self.sign_selected = true;
                }
                (Phase::Decision, Message::Select) => {
                    if self.sign_selected {
                        if _effects
                            .send(SignerMessage::ConfirmDemo { token: self.token })
                            .is_ok()
                        {
                            self.phase = Phase::Signing;
                            self.status = "Signing demo...";
                            self.pages.clear();
                        }
                    } else {
                        self.cancel(_effects);
                    }
                }
                (Phase::Signing, Message::DemoSigned { token, ok }) if token == self.token => {
                    self.phase = Phase::Idle;
                    self.status = if ok {
                        "Signed! USB hex"
                    } else {
                        "Failed - retry left"
                    };
                }
                _ => {}
            }
        }
        None
    }
    #[cfg(feature = "psbt-demo")]
    fn cancel(&mut self, effects: &mut impl crate::signals::Effects) {
        if effects
            .send(SignerMessage::CancelDemo { token: self.token })
            .is_ok()
        {
            self.phase = Phase::Idle;
            self.pages.clear();
            self.sign_selected = false;
            self.status = "Cancelled - no sign";
        }
    }
}
impl Default for HomeScreen {
    fn default() -> Self {
        Self::new()
    }
}
