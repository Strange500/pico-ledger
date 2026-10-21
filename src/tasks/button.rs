use embassy_rp::gpio::Input;
use embassy_time::Timer;

use crate::signals::BUTTON_SIGNAL;

/// Button task on GP2 (active-low with internal pull-up).
#[embassy_executor::task]
pub async fn button_task(mut button: Input<'static>) {
    loop {
        button.wait_for_falling_edge().await;
        Timer::after_millis(30).await;
        if button.is_low() {
            log_info!("Button (GP2) clicked");
            BUTTON_SIGNAL.signal(());
            button.wait_for_rising_edge().await;
            Timer::after_millis(30).await;
        }
    }
}
