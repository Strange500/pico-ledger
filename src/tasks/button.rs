use crate::signals::{Message, MESSAGE_QUEUE};
use embassy_futures::select::{select, Either};
use embassy_rp::gpio::Input;
use embassy_time::Timer;

#[embassy_executor::task]
pub async fn dual_button_task(mut btn_left: Input<'static>, mut btn_right: Input<'static>) {
    loop {
        // 1. Sleep the CPU until EITHER button generates a falling edge
        match select(
            btn_left.wait_for_falling_edge(),
            btn_right.wait_for_falling_edge(),
        )
        .await
        {
            Either::First(()) | Either::Second(()) => {
                // Stamp the frame visible at the FIRST physical edge, not after
                // debounce or queue wait. Old queued presses cannot confirm a
                // screen that was not yet visible when the press began.
                #[cfg(feature = "psbt-demo")]
                let frame =
                    crate::signals::DISPLAY_FRAME.load(core::sync::atomic::Ordering::Relaxed);
                // 2. The Grace Period
                // Wait 50ms to debounce the mechanical switch and give the
                // user's other finger time to complete the simultaneous press.
                Timer::after_millis(50).await;

                // 3. Read the actual physical state of both pins
                let left_pressed = btn_left.is_low();
                let right_pressed = btn_right.is_low();

                // 4. Evaluate the combined state and send exactly one Message
                let message = if left_pressed && right_pressed {
                    Some(Message::Select)
                } else if left_pressed {
                    Some(Message::Back)
                } else if right_pressed {
                    Some(Message::Next)
                } else {
                    None
                };
                if let Some(message) = message {
                    #[cfg(feature = "psbt-demo")]
                    let message = Message::PhysicalButton {
                        frame,
                        button: match message {
                            Message::Back => pico_ledger_core::signals::PhysicalButton::Left,
                            Message::Next => pico_ledger_core::signals::PhysicalButton::Right,
                            _ => pico_ledger_core::signals::PhysicalButton::Both,
                        },
                    };
                    MESSAGE_QUEUE.send(message).await;
                }

                // 5. Wait for the user to physically release both buttons
                // This prevents rapid-fire events if they hold the buttons down.
                while btn_left.is_low() || btn_right.is_low() {
                    Timer::after_millis(10).await;
                }

                // Debounce the release
                Timer::after_millis(50).await;
            }
        }
    }
}
