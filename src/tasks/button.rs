use embassy_rp::gpio::Input;
use embassy_time::Timer;

use crate::signals::{Message, MESSAGE_QUEUE};

#[embassy_executor::task]
pub async fn button_next_task(mut button: Input<'static>) {
    loop {
        button.wait_for_falling_edge().await;
        Timer::after_millis(30).await;
        if button.is_low() {
            log_info!("Button Next (GP2) clicked");
            MESSAGE_QUEUE.send(Message::Next).await;
            button.wait_for_rising_edge().await;
            Timer::after_millis(30).await;
        }
    }
}

#[embassy_executor::task]
pub async fn button_back_task(mut button: Input<'static>) {
    loop {
        button.wait_for_falling_edge().await;
        Timer::after_millis(30).await;
        if button.is_low() {
            log_info!("Back button (GP3) clicked");
            MESSAGE_QUEUE.send(Message::Back).await;
            button.wait_for_rising_edge().await;
            Timer::after_millis(30).await;
        }
    }
}
