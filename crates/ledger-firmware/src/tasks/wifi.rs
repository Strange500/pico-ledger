use cyw43_pio::PioSpi;
use embassy_rp::gpio::Output;
use embassy_rp::peripherals::PIO0;

/// Background task running the CYW43 Wi-Fi/Bluetooth controller driver.
/// Required on Pico 2 W/WH — the on-board LED is driven through this chip.
#[embassy_executor::task]
pub async fn cyw43_task(
    runner: cyw43::Runner<'static, cyw43::SpiBus<Output<'static>, PioSpi<'static, PIO0, 0>>>,
) -> ! {
    runner.run().await
}
