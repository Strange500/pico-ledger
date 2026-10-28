use embassy_futures::select::{select, Either};
use embassy_time::{Duration, Timer};

use crate::signals::{Message, ACTIVITY_SIGNAL, MESSAGE_QUEUE};

#[embassy_executor::task]
pub async fn lock_task() -> ! {
    loop {
        match select(
            Timer::after(Duration::from_secs(10)),
            ACTIVITY_SIGNAL.wait(),
        )
        .await
        {
            Either::First(()) => {
                MESSAGE_QUEUE.send(Message::Lock).await;
                log_info!("Locking the device due to inactivity");
            }
            Either::Second(()) => {
                // Activity signal received, reset the timer
                // You can add your own logic here to reset the timer
            }
        }
    }
}
