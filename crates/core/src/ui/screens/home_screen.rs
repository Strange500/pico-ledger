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

use crate::{
    psbt::TransactionReview,
    signals::{Message, SignerMessage},
    ui::state::AppModel,
};

#[cfg(not(feature = "test-mode"))]
const HOME_HEADER: &str = "Pico Ledger";
#[cfg(feature = "test-mode")]
const HOME_HEADER: &str = "UNSAFE TEST MODE";

struct ReviewPage {
    title: String,
    body: String,
}

enum Phase {
    Idle,
    Reviewing,
    Confirming,
    Signing,
    DescriptorConfirmation,
}

pub struct HomeScreen {
    phase: Phase,
    request_id: u16,
    pages: Vec<ReviewPage>,
    page: usize,
    status: &'static str,
}

fn review_pages(review: &TransactionReview) -> Vec<ReviewPage> {
    let mut pages = Vec::new();
    let mut add = |title: String, value: String| {
        for (part, chunk) in value.as_bytes().chunks(60).enumerate() {
            let mut body = String::new();
            for (line, text) in chunk.chunks(20).enumerate() {
                if line != 0 {
                    body.push('\n');
                }
                body.push_str(core::str::from_utf8(text).unwrap_or("INVALID"));
            }
            pages.push(ReviewPage {
                title: if value.len() > 60 {
                    format!("{title} {}", part + 1)
                } else {
                    title.clone()
                },
                body,
            });
        }
    };

    add("Network".into(), "Bitcoin testnet BIP84".into());
    add("Transaction ID".into(), review.txid.to_string());
    add(
        "Transaction".into(),
        format!(
            "version {} lock {} inputs {}",
            review.version, review.lock_time, review.input_count
        ),
    );
    for (index, sequence) in review.sequences.iter().enumerate() {
        add(
            format!("Input {} sequence", index + 1),
            sequence.to_string(),
        );
    }
    for (index, output) in review.outputs.iter().enumerate() {
        add(
            format!("Output {} address", index + 1),
            output.address.clone(),
        );
        add(
            format!("Output {}", index + 1),
            format!(
                "{} sat {}",
                output.amount_sat,
                if output.is_change {
                    "change"
                } else {
                    "recipient"
                }
            ),
        );
    }
    add(
        "Total input".into(),
        format!("{} sat", review.total_input_sat),
    );
    add(
        "Total output".into(),
        format!("{} sat", review.total_output_sat),
    );
    add("Network fee".into(), format!("{} sat", review.fee_sat));
    add("Signature".into(), "SIGHASH_ALL".into());
    pages
}

impl HomeScreen {
    pub fn new() -> Self {
        Self {
            phase: Phase::Idle,
            request_id: 0,
            pages: Vec::new(),
            page: 0,
            status: "Waiting for USB PSBT",
        }
    }

    pub fn view(&self) -> impl View<BinaryColor, AppModel> {
        let (header, title, body, controls) = match self.phase {
            Phase::Idle | Phase::Signing => (
                HOME_HEADER.into(),
                "USB PSBT".into(),
                self.status.into(),
                String::new(),
            ),
            Phase::Reviewing => {
                let page = &self.pages[self.page];
                (
                    format!("Review {}/{}", self.page + 1, self.pages.len()),
                    page.title.clone(),
                    page.body.clone(),
                    "L:back R:next B:no".into(),
                )
            }
            Phase::Confirming => (
                "Final confirmation".into(),
                "Sign this PSBT?".into(),
                "Both buttons: SIGN".into(),
                "L:cancel R:review".into(),
            ),
            Phase::DescriptorConfirmation => (
                "Privacy approval".into(),
                "Export watch-only?".into(),
                "Host sees addresses".into(),
                "Both:allow L:cancel".into(),
            ),
        };

        VStack::new((
            Text::new(header, &FONT_6X10),
            Text::new(title, &FONT_6X10),
            Text::new(body, &FONT_6X10),
            Text::new(controls, &FONT_6X10),
            Spacer::default(),
        ))
        .padding(Edges::Horizontal, 4)
        .padding(Edges::Vertical, 2)
    }

    pub fn update(
        &mut self,
        message: Message,
        effects: &mut impl crate::signals::Effects,
    ) -> Option<AppModel> {
        match (&self.phase, message) {
            (Phase::Idle, Message::PsbtReview { request_id, review }) => {
                self.request_id = request_id;
                self.pages = review_pages(&review);
                self.page = 0;
                self.phase = Phase::Reviewing;
            }
            (Phase::Reviewing, Message::Next) => {
                if self.page + 1 == self.pages.len() {
                    self.phase = Phase::Confirming;
                } else {
                    self.page += 1;
                }
            }
            (Phase::Reviewing, Message::Back) => {
                self.page = self.page.saturating_sub(1);
            }
            (Phase::Reviewing, Message::Select) | (Phase::Confirming, Message::Back) => {
                if effects
                    .send(SignerMessage::RejectPsbt {
                        request_id: self.request_id,
                    })
                    .is_ok()
                {
                    self.phase = Phase::Idle;
                    self.pages.clear();
                    self.status = "Transaction rejected";
                }
            }
            (Phase::Confirming, Message::Next) => {
                self.page = self.pages.len().saturating_sub(1);
                self.phase = Phase::Reviewing;
            }
            (Phase::Confirming, Message::Select) => {
                if effects
                    .send(SignerMessage::ApprovePsbt {
                        request_id: self.request_id,
                    })
                    .is_ok()
                {
                    self.phase = Phase::Signing;
                    self.pages.clear();
                    self.status = "Signing...";
                }
            }
            (Phase::Signing, Message::PsbtSigned { request_id })
                if request_id == self.request_id =>
            {
                self.phase = Phase::Idle;
                self.status = "Signed PSBT sent";
            }
            (_, Message::PsbtFailed { error, .. }) => {
                self.phase = Phase::Idle;
                self.pages.clear();
                self.status = error;
            }
            (_, Message::PsbtCancelled { request_id }) if request_id == self.request_id => {
                self.phase = Phase::Idle;
                self.pages.clear();
                self.status = "Transaction rejected";
            }
            (Phase::Idle, Message::DescriptorReview { request_id }) => {
                self.request_id = request_id;
                self.phase = Phase::DescriptorConfirmation;
            }
            (Phase::DescriptorConfirmation, Message::Select) => {
                if effects
                    .send(SignerMessage::ApproveDescriptor {
                        request_id: self.request_id,
                    })
                    .is_ok()
                {
                    self.phase = Phase::Idle;
                    self.status = "Exporting descriptor";
                }
            }
            (Phase::DescriptorConfirmation, Message::Back) => {
                if effects
                    .send(SignerMessage::RejectDescriptor {
                        request_id: self.request_id,
                    })
                    .is_ok()
                {
                    self.phase = Phase::Idle;
                    self.status = "Export rejected";
                }
            }
            (Phase::Idle, Message::DescriptorExported) => {
                self.status = "Descriptor exported";
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
