//! Hardware review state-machine regression tests (public fixture only).
use crate::{
    demo::DemoReview,
    signals::{Message, SignerMessage},
    tests::Recorder,
    ui::{screens::home_screen::HomeScreen, state::AppModel},
};

#[test]
fn queued_presses_from_older_frames_cannot_confirm() {
    use crate::signals::PhysicalButton;
    for (button, expected) in [
        (PhysicalButton::Left, Message::Back),
        (PhysicalButton::Right, Message::Next),
        (PhysicalButton::Both, Message::Select),
    ] {
        let press = Message::PhysicalButton { button, frame: 10 };
        assert_eq!(press.clone().for_display_frame(10), Some(expected));
        assert_eq!(press.clone().for_display_frame(11), None);
        assert_eq!(press.clone().for_display_frame(0), None);
        assert_eq!(press.for_display_frame(u32::MAX), None);
    }
    assert_eq!(Message::Lock.for_display_frame(11), Some(Message::Lock));
}

#[test]
fn review_requires_every_page_and_explicit_confirm() {
    let pending = DemoReview::prepare(&mut [42; 32]).unwrap();
    let mut home = HomeScreen::new();
    let mut effects = Recorder::default();
    home.update(Message::Back, &mut effects);
    assert_eq!(effects.commands, [SignerMessage::PrepareDemo]);
    home.update(
        Message::DemoPrepared {
            token: 7,
            review: Ok(pending.review().clone()),
        },
        &mut effects,
    );
    home.update(Message::Select, &mut effects); // Invisible/failed flush cannot act.
    assert_eq!(effects.commands.len(), 1);
    let count = home.review_pages().len();
    assert!(count > 10);
    for _ in 0..count {
        home.displayed();
        home.update(Message::Next, &mut effects);
    }
    assert_eq!(effects.commands.len(), 1); // Paging does not sign.
    home.displayed();
    home.update(Message::Select, &mut effects); // Default is CANCEL.
    assert_eq!(
        effects.commands.last(),
        Some(&SignerMessage::CancelDemo { token: 7 })
    );
}

#[test]
fn confirmed_review_signs_once_and_lock_discards_ui() {
    let pending = DemoReview::prepare(&mut [42; 32]).unwrap();
    let mut home = HomeScreen::new();
    let mut effects = Recorder::default();
    home.update(Message::Back, &mut effects);
    home.update(
        Message::DemoPrepared {
            token: 9,
            review: Ok(pending.review().clone()),
        },
        &mut effects,
    );
    let pages = home.review_pages().len();
    for _ in 0..pages {
        home.displayed();
        home.update(Message::Next, &mut effects);
    }
    home.displayed();
    home.update(Message::Next, &mut effects); // Explicitly choose SIGN.
    effects.full = true;
    home.displayed();
    home.update(Message::Select, &mut effects);
    assert_eq!(effects.commands.len(), 1);
    effects.full = false;
    home.displayed();
    home.update(Message::Select, &mut effects);
    home.update(Message::Select, &mut effects);
    assert_eq!(
        effects.commands.last(),
        Some(&SignerMessage::ConfirmDemo { token: 9 })
    );
    assert_eq!(effects.commands.len(), 2);
    let mut app = AppModel::Home(home);
    app.update(Message::Lock, &mut effects);
    assert!(matches!(app, AppModel::Lock(_)));
    app.update(Message::DemoSigned { token: 9, ok: true }, &mut effects);
    assert!(matches!(app, AppModel::Lock(_)));
}

#[test]
fn session_rejects_early_stale_cancelled_locked_and_duplicate_confirms() {
    let mut session = crate::demo::DemoSession::default();
    assert!(session.confirm(1).is_none());
    let (first, _) = session.prepare(&mut [41; 32]).unwrap();
    assert!(session.confirm(first + 1).is_none());
    session.cancel(first);
    assert!(session.confirm(first).is_none());
    let (second, _) = session.prepare(&mut [42; 32]).unwrap();
    assert_ne!(first, second);
    assert!(session.confirm(first).is_none());
    session.cancel(first); // A stale cancel cannot cancel another review.
    assert!(session.confirm(second).is_some());
    assert!(session.confirm(second).is_none());
    let (third, _) = session.prepare(&mut [43; 32]).unwrap();
    session.lock();
    assert!(session.confirm(third).is_none());
}

#[test]
fn pages_preserve_full_identifiers_and_fit_oled() {
    use alloc::string::ToString;
    let pending = DemoReview::prepare(&mut [42; 32]).unwrap();
    let review = pending.review().clone();
    let mut home = HomeScreen::new();
    let mut effects = Recorder::default();
    home.update(Message::Back, &mut effects);
    home.update(
        Message::DemoPrepared {
            token: 1,
            review: Ok(review.clone()),
        },
        &mut effects,
    );
    for page in home.review_pages() {
        assert!(page.title.len() <= 20, "{}", page.title);
        assert!(page.body.lines().count() <= 3);
        assert!(page.body.lines().all(|line| line.len() <= 20));
    }
    let value = |prefix: &str| -> alloc::string::String {
        home.review_pages()
            .iter()
            .filter(|p| p.title.starts_with(prefix))
            .flat_map(|p| p.body.lines())
            .collect()
    };
    assert_eq!(value("Transaction ID"), review.txid.to_string());
    assert_eq!(value("In1 outpoint"), review.inputs[0].outpoint.to_string());
    assert_eq!(value("In1 address"), review.inputs[0].address);
    assert_eq!(value("Out1 address"), review.outputs[0].address);
    assert_eq!(value("Fee sats"), "1000");
    home.displayed();
    home.update(Message::Back, &mut effects); // Clamps at the first page.
    home.displayed();
    home.update(Message::Select, &mut effects); // Cancel from the very first page.
    assert_eq!(
        effects.commands.last(),
        Some(&SignerMessage::CancelDemo { token: 1 })
    );
}

#[test]
fn all_review_and_confirmation_frames_render_on_128x64() {
    use buoyant::{app::App, primitives::Size, render_target::EmbeddedGraphicsRenderTarget};
    use embedded_graphics::{pixelcolor::BinaryColor, prelude::*, Pixel};
    struct Oled {
        pixels: usize,
    }
    impl OriginDimensions for Oled {
        fn size(&self) -> embedded_graphics::geometry::Size {
            embedded_graphics::geometry::Size::new(128, 64)
        }
    }
    impl DrawTarget for Oled {
        type Color = BinaryColor;
        type Error = core::convert::Infallible;
        fn draw_iter<I: IntoIterator<Item = Pixel<BinaryColor>>>(
            &mut self,
            pixels: I,
        ) -> Result<(), Self::Error> {
            for Pixel(point, color) in pixels {
                assert!(
                    (0..128).contains(&point.x) && (0..64).contains(&point.y),
                    "out of OLED: {point:?}"
                );
                if color == BinaryColor::On {
                    self.pixels += 1;
                }
            }
            Ok(())
        }
    }
    fn view(state: &AppModel) -> impl buoyant::view::View<BinaryColor, AppModel> + use<> {
        state.view()
    }
    let pending = DemoReview::prepare(&mut [42; 32]).unwrap();
    let mut home = HomeScreen::new();
    let mut effects = Recorder::default();
    home.update(Message::Back, &mut effects);
    home.update(
        Message::DemoPrepared {
            token: 1,
            review: Ok(pending.review().clone()),
        },
        &mut effects,
    );
    let count = home.review_pages().len();
    let mut app = App::new(AppModel::Home(home), Size::new(128, 64), view);
    for _ in 0..count + 2 {
        let mut oled = Oled { pixels: 0 };
        {
            let mut target = EmbeddedGraphicsRenderTarget::new(&mut oled);
            app.render_animated(&mut target, &BinaryColor::On);
        }
        assert!(oled.pixels > 50);
        app.state_mut().displayed();
        app.state_mut().update(Message::Next, &mut effects);
    }
    assert_eq!(effects.commands, [SignerMessage::PrepareDemo]);
    app.state_mut().update(Message::Select, &mut effects); // No new flush -> ignored.
    assert_eq!(effects.commands.len(), 1);
    app.state_mut().displayed();
    app.state_mut().update(Message::Select, &mut effects);
    assert_eq!(
        effects.commands.last(),
        Some(&SignerMessage::ConfirmDemo { token: 1 })
    );
}

#[test]
fn immutable_review_matches_signed_transaction_and_fields() {
    let pending = DemoReview::prepare(&mut [42; 32]).unwrap();
    let reviewed = pending.review().clone();
    assert_eq!(reviewed.inputs.len(), 1);
    assert_eq!(reviewed.inputs[0].amount_sat, 100_000);
    let signed = pending.sign(&mut [43; 32]).unwrap();
    let psbt = bitcoin::psbt::Psbt::deserialize(&signed).unwrap();
    assert_eq!(psbt.unsigned_tx.compute_txid(), reviewed.txid);
    assert_eq!(psbt.inputs[0].partial_sigs.len(), 1);
    assert_eq!(
        psbt.unsigned_tx.output[0].value.to_sat(),
        reviewed.outputs[0].amount_sat
    );
}
