#[cfg(feature = "psbt-demo")]
use pico_ledger_core::ui::state::AppModel;
use pico_ledger_core::{
    signals::{CommandQueueFull, Effects, Message, SignerMessage},
    ui::screens::home_screen::HomeScreen,
};

#[derive(Default)]
struct Fake {
    commands: Vec<SignerMessage>,
    full: bool,
}
impl Effects for Fake {
    fn send(&mut self, command: SignerMessage) -> Result<(), CommandQueueFull> {
        if self.full {
            return Err(CommandQueueFull);
        }
        self.commands.push(command);
        Ok(())
    }
    fn activity(&mut self) {}
}

#[test]
fn ordinary_buttons_never_request_signing() {
    let mut home = HomeScreen::new();
    let mut effects = Fake::default();
    for msg in [Message::Next, Message::Select] {
        home.update(msg, &mut effects);
    }
    #[cfg(not(feature = "psbt-demo"))]
    home.update(Message::Back, &mut effects);
    assert!(effects.commands.is_empty());
}

#[cfg(feature = "psbt-demo")]
#[test]
fn left_press_is_single_flight_and_completion_allows_retry() {
    let mut home = HomeScreen::new();
    let mut effects = Fake::default();
    home.update(Message::DemoSigned { token: 1, ok: true }, &mut effects); // unsolicited result ignored
    home.update(Message::Back, &mut effects);
    home.update(Message::Back, &mut effects);
    assert_eq!(effects.commands, [SignerMessage::PrepareDemo]);
    home.update(
        Message::DemoPrepared {
            token: 1,
            review: Err("Invalid fixture"),
        },
        &mut effects,
    );
    home.update(Message::Back, &mut effects);
    assert_eq!(effects.commands.len(), 2);
    home.update(
        Message::DemoPrepared {
            token: 2,
            review: Err("Invalid fixture"),
        },
        &mut effects,
    );
    home.update(Message::Back, &mut effects);
    assert_eq!(effects.commands.len(), 3);
}

#[cfg(feature = "psbt-demo")]
#[test]
fn full_queue_does_not_stick_busy_and_lock_blocks_new_requests() {
    let mut app = AppModel::Home(HomeScreen::new());
    let mut effects = Fake {
        full: true,
        ..Fake::default()
    };
    app.update(Message::Back, &mut effects);
    effects.full = false;
    app.update(Message::Back, &mut effects);
    assert_eq!(effects.commands, [SignerMessage::PrepareDemo]);
    app.update(Message::Lock, &mut effects);
    app.update(Message::DemoSigned { token: 1, ok: true }, &mut effects);
    app.update(Message::Back, &mut effects);
    assert!(matches!(app, AppModel::Lock(_)));
    assert_eq!(
        effects.commands,
        [SignerMessage::PrepareDemo, SignerMessage::Lock]
    );
}
