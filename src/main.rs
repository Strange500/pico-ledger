#![no_std]
#![no_main]

use cyw43_pio::{PioSpi, DEFAULT_CLOCK_DIVIDER};
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use embassy_rp::gpio::{Input, Level, Output, Pull};
use embassy_rp::peripherals::{DMA_CH0, PIO0, USB};
use embassy_rp::pio::{InterruptHandler as PioInterruptHandler, Pio};
use embassy_rp::usb::{Driver, InterruptHandler as UsbInterruptHandler};
use embassy_rp::{bind_interrupts, dma};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};
use panic_probe as _;
use static_cell::StaticCell;

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => PioInterruptHandler<PIO0>;
    DMA_IRQ_0 => dma::InterruptHandler<DMA_CH0>;
    USBCTRL_IRQ => UsbInterruptHandler<USB>;
});

/// Log simultaneously to both defmt (SWD RTT) and USB CDC-ACM serial.
macro_rules! log_info {
    ($($arg:tt)*) => {
        ::defmt::info!($($arg)*);
        ::log::info!($($arg)*);
    };
}

static BUTTON_SIGNAL: Signal<CriticalSectionRawMutex, ()> = Signal::new();

/// Background task running the USB serial logger (CDC-ACM over USB cable).
#[embassy_executor::task]
async fn logger_task(driver: Driver<'static, USB>) {
    embassy_usb_logger::run!(1024, log::LevelFilter::Info, driver);
}

/// Background task running the CYW43 Wi-Fi/Bluetooth controller driver.
#[embassy_executor::task]
async fn cyw43_task(
    runner: cyw43::Runner<'static, cyw43::SpiBus<Output<'static>, PioSpi<'static, PIO0, 0>>>,
) -> ! {
    runner.run().await
}

/// Button task on GP2 (active-low with internal pull-up).
#[embassy_executor::task]
async fn button_task(mut button: Input<'static>) {
    loop {
        // Wait for button press (falling edge)
        button.wait_for_falling_edge().await;
        Timer::after_millis(30).await;
        if button.is_low() {
            log_info!("Button (GP2) clicked: toggling LED state");
            BUTTON_SIGNAL.signal(());

            // Wait for release
            button.wait_for_rising_edge().await;
            Timer::after_millis(30).await;
        }
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    // 1. Initialize USB CDC-ACM Serial Logger first so logs can stream immediately
    let usb_driver = Driver::new(p.USB, Irqs);
    spawner.spawn(logger_task(usb_driver).unwrap());

    log_info!("RP2350 (Pico 2 W) initialized - USB Serial logging active");

    // 2. Static buffers for CYW43 Wi-Fi co-processor firmware and NVRAM
    static FW: cyw43::Aligned<cyw43::A4, [u8; 231077]> =
        cyw43::Aligned(*cyw43_firmware::CYW43_43439A0);
    static CLM: cyw43::Aligned<cyw43::A4, [u8; 984]> =
        cyw43::Aligned(*cyw43_firmware::CYW43_43439A0_CLM);
    static NVRAM: cyw43::Aligned<cyw43::A4, [u8; 742]> =
        cyw43::Aligned(*include_bytes!("../firmware/nvram_rp2040.bin"));

    // 3. Configure CYW43 power and SPI control pins
    let pwr = Output::new(p.PIN_23, Level::Low);
    let cs = Output::new(p.PIN_25, Level::High);
    let mut pio = Pio::new(p.PIO0, Irqs);
    let spi = PioSpi::new(
        &mut pio.common,
        pio.sm0,
        DEFAULT_CLOCK_DIVIDER,
        pio.irq0,
        cs,
        p.PIN_24,
        p.PIN_29,
        dma::Channel::new(p.DMA_CH0, Irqs),
    );

    static STATE: StaticCell<cyw43::State> = StaticCell::new();
    let state = STATE.init(cyw43::State::new());
    let (_net_device, mut control, runner) = cyw43::new(state, pwr, spi, &FW, &NVRAM).await;
    spawner.spawn(cyw43_task(runner).unwrap());
    control.init(&*CLM).await;

    // 4. Spawn button monitoring on GP2
    let button = Input::new(p.PIN_2, Pull::Up);
    spawner.spawn(button_task(button).unwrap());

    log_info!("Pico 2 W ready: LED ON by default. Press button to deactivate/reactivate.");

    // Start with LED ON by default
    let mut is_active = true;
    let mut led_on = true;
    control.gpio_set(0, led_on).await;

    let delay = Duration::from_millis(500);

    loop {
        if is_active {
            // Blink the LED while active; immediately interrupt if button is clicked
            match select(Timer::after(delay), BUTTON_SIGNAL.wait()).await {
                Either::First(_) => {
                    led_on = !led_on;
                    control.gpio_set(0, led_on).await;
                }
                Either::Second(_) => {
                    // Button clicked: deactivate LED
                    is_active = false;
                    led_on = false;
                    control.gpio_set(0, false).await;
                    log_info!("LED deactivated (OFF)");
                }
            }
        } else {
            // Inactive: wait for button click to reactivate
            BUTTON_SIGNAL.wait().await;
            is_active = true;
            led_on = true;
            control.gpio_set(0, true).await;
            log_info!("LED reactivated (ON)");
        }
    }
}
