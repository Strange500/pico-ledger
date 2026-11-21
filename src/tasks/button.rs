use core::sync::atomic::Ordering;

use crate::signals::{Message, DISPLAY_FRAME, MESSAGE_QUEUE};
use embassy_futures::select::{select, Either};
use embassy_rp::gpio::Input;
use embassy_time::Timer;
use pico_ledger_core::signals::PhysicalButton;

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
                let frame = DISPLAY_FRAME.load(Ordering::Acquire);
                // 2. The Grace Period
                // Wait 50ms to debounce the mechanical switch and give the
                // user's other finger time to complete the simultaneous press.
                Timer::after_millis(50).await;

                // 3. Read the actual physical state of both pins
                let left_pressed = btn_left.is_low();
                let right_pressed = btn_right.is_low();

                // 4. Evaluate the combined state and send exactly one Message
                let button = if left_pressed && right_pressed {
                    Some(PhysicalButton::Both)
                } else if left_pressed {
                    Some(PhysicalButton::Left)
                } else if right_pressed {
                    Some(PhysicalButton::Right)
                } else {
                    None
                };
                if let Some(button) = button {
                    MESSAGE_QUEUE
                        .send(Message::PhysicalButton { button, frame })
                        .await;
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
