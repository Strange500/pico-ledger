use embassy_rp::peripherals::USB;
use embassy_rp::usb::Driver;

/// Background task running the USB serial logger (CDC-ACM over USB cable).
#[embassy_executor::task]
pub async fn logger_task(driver: Driver<'static, USB>) {
    embassy_usb_logger::run!(1024, log::LevelFilter::Info, driver);
}
