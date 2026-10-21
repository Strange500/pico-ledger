use embassy_futures::select::{select, Either};
use embassy_rp::peripherals::TRNG;
use embassy_rp::trng::Trng;
use embassy_time::{Duration, Timer};

use crate::signals::{BUTTON_SIGNAL, WORD_SIGNAL};
use crate::wordlist::WORD_LIST;

/// Background task: generate a random BIP-39 word every 5 s using hardware TRNG,
/// or immediately whenever triggered by the button.
#[embassy_executor::task]
pub async fn rd_word(mut trng: Trng<'static, TRNG>) {
    loop {
        let mut buf = [0u8; 2];
        trng.fill_bytes(&mut buf).await;
        let word = WORD_LIST[(u16::from_le_bytes(buf) as usize) % WORD_LIST.len()];
        log_info!("Random word: {}", word);
        WORD_SIGNAL.signal(word);

        // Wait 5 seconds OR until a button press signals to pick a new word immediately
        match select(Timer::after(Duration::from_secs(5)), BUTTON_SIGNAL.wait()).await {
            Either::First(_) => {}
            Either::Second(_) => {}
        }
    }
}
