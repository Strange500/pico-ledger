#![no_std]
#![no_main]

use cyw43_pio::{PioSpi, DEFAULT_CLOCK_DIVIDER};
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use embassy_rp::gpio::{Input, Level, Output, Pull};
use embassy_rp::i2c::{Config as I2cConfig, I2c, InterruptHandler as I2cInterruptHandler};
use embassy_rp::peripherals::{DMA_CH0, I2C0, PIO0, TRNG, USB};
use embassy_rp::pio::{InterruptHandler as PioInterruptHandler, Pio};
use embassy_rp::trng::{Config as TrngConfig, InterruptHandler as TrngInterruptHandler, Trng};
use embassy_rp::usb::{Driver, InterruptHandler as UsbInterruptHandler};
use embassy_rp::{bind_interrupts, dma};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};
use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Line, PrimitiveStyle},
    text::Text,
};
use panic_probe as _;
use ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306};
use static_cell::StaticCell;

mod wordlist;
use wordlist::WORD_LIST;
extern crate alloc;

use embedded_alloc::LlffHeap as Heap;

#[global_allocator]
static HEAP: Heap = Heap::empty();

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => PioInterruptHandler<PIO0>;
    DMA_IRQ_0 => dma::InterruptHandler<DMA_CH0>;
    USBCTRL_IRQ => UsbInterruptHandler<USB>;
    TRNG_IRQ => TrngInterruptHandler<TRNG>;
    I2C0_IRQ => I2cInterruptHandler<I2C0>;
});

/// Log simultaneously to both defmt (SWD RTT) and USB CDC-ACM serial.
macro_rules! log_info {
    ($($arg:tt)*) => {{
        ::defmt::info!($($arg)*);
        ::log::info!($($arg)*);
    }};
}

static BUTTON_SIGNAL: Signal<CriticalSectionRawMutex, ()> = Signal::new();
static WORD_SIGNAL: Signal<CriticalSectionRawMutex, &'static str> = Signal::new();
static CHANGE_WORD_SIGNAL: Signal<CriticalSectionRawMutex, ()> = Signal::new();

/// Background task running the USB serial logger (CDC-ACM over USB cable).
#[embassy_executor::task]
async fn logger_task(driver: Driver<'static, USB>) {
    embassy_usb_logger::run!(1024, log::LevelFilter::Info, driver);
}

/// Background task running the CYW43 Wi-Fi/Bluetooth controller driver.
/// Required on Pico 2 W/WH — the on-board LED is driven through this chip.
#[embassy_executor::task]
async fn cyw43_task(
    runner: cyw43::Runner<'static, cyw43::SpiBus<Output<'static>, PioSpi<'static, PIO0, 0>>>,
) -> ! {
    runner.run().await
}

/// Background task: generate a random BIP-39 word every 5 s using hardware TRNG,
/// or immediately whenever triggered by the button.
#[embassy_executor::task]
async fn rd_word(mut trng: Trng<'static, TRNG>) {
    loop {
        let mut buf = [0u8; 2];
        trng.fill_bytes(&mut buf).await;
        let word = WORD_LIST[(u16::from_le_bytes(buf) as usize) % WORD_LIST.len()];
        log_info!("Random word: {}", word);
        WORD_SIGNAL.signal(word);

        // Wait 5 seconds OR until a button press signals to pick a new word immediately
        match select(
            Timer::after(Duration::from_secs(5)),
            CHANGE_WORD_SIGNAL.wait(),
        )
        .await
        {
            Either::First(_) => {}
            Either::Second(_) => {}
        }
    }
}

/// Background task: drive the SSD1306 128x64 OLED over I2C (GP4 SDA, GP5 SCL).
#[embassy_executor::task]
async fn display_task(i2c: I2c<'static, I2C0, embassy_rp::i2c::Async>) {
    let interface = I2CDisplayInterface::new(i2c);
    let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();

    // Retry init a few times — the display may not be ready immediately after power-on.
    let mut initialized = false;
    for _ in 0..5 {
        if display.init().is_ok() {
            initialized = true;
            break;
        }
        log_info!("SSD1306 init failed, retrying in 500 ms...");
        Timer::after(Duration::from_millis(500)).await;
    }

    if !initialized {
        log_info!("SSD1306 not detected — check SDA=GP4, SCL=GP5, VCC=3V3, GND");
        return;
    }
    log_info!("SSD1306 initialized successfully.");

    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X10)
        .text_color(BinaryColor::On)
        .build();
    let line_style = PrimitiveStyle::with_stroke(BinaryColor::On, 1);

    let mut render = |status: &str, line1: &str, line2: &str| {
        display.clear_buffer();
        let _ = Text::new("PICO 2 LEDGER", Point::new(24, 10), text_style).draw(&mut display);
        let _ = Line::new(Point::new(0, 14), Point::new(127, 14))
            .into_styled(line_style)
            .draw(&mut display);
        let _ = Text::new(status, Point::new(4, 28), text_style).draw(&mut display);
        let _ = Text::new(line1, Point::new(4, 42), text_style).draw(&mut display);
        if !line2.is_empty() {
            let _ = Text::new(line2, Point::new(4, 56), text_style).draw(&mut display);
        }
        let _ = display.flush();
    };

    render("Status: Ready", "Waiting for word...", "");

    loop {
        let word = WORD_SIGNAL.wait().await;
        render("Status: Active", "Random Word:", word);
    }
}

/// Button task on GP2 (active-low with internal pull-up).
#[embassy_executor::task]
async fn button_task(mut button: Input<'static>) {
    loop {
        button.wait_for_falling_edge().await;
        Timer::after_millis(30).await;
        if button.is_low() {
            log_info!("Button (GP2) clicked");
            CHANGE_WORD_SIGNAL.signal(());
            button.wait_for_rising_edge().await;
            Timer::after_millis(30).await;
        }
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    unsafe {
        embedded_alloc::init!(HEAP, 128 * 1024);
    }

    let p = embassy_rp::init(Default::default());

    // 1. USB CDC-ACM serial logger
    let usb_driver = Driver::new(p.USB, Irqs);
    spawner.spawn(logger_task(usb_driver).unwrap());

    log_info!("RP2350 (Pico 2 WH) booted - USB serial active");

    // 2. SSD1306 OLED over I2C0 — GP4 = SDA, GP5 = SCL
    let mut i2c_config = I2cConfig::default();
    i2c_config.frequency = 400_000;
    let i2c = I2c::new_async(p.I2C0, p.PIN_5, p.PIN_4, Irqs, i2c_config);
    spawner.spawn(display_task(i2c).unwrap());

    // 3. Hardware TRNG → random BIP-39 word every 5 s (or on button press)
    let trng = Trng::new(p.TRNG, Irqs, TrngConfig::default());
    spawner.spawn(rd_word(trng).unwrap());

    // 4. Button on GP2
    let button = Input::new(p.PIN_2, Pull::Up);
    spawner.spawn(button_task(button).unwrap());

    // 5. CYW43 — Pico 2 W/WH on-board LED is behind this chip (not a plain GPIO)
    static FW: cyw43::Aligned<cyw43::A4, [u8; 231077]> =
        cyw43::Aligned(*cyw43_firmware::CYW43_43439A0);
    static CLM: cyw43::Aligned<cyw43::A4, [u8; 984]> =
        cyw43::Aligned(*cyw43_firmware::CYW43_43439A0_CLM);
    static NVRAM: cyw43::Aligned<cyw43::A4, [u8; 742]> =
        cyw43::Aligned(*include_bytes!("../firmware/nvram_rp2040.bin"));

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
    control
        .set_power_management(cyw43::PowerManagementMode::None)
        .await;

    log_info!("Pico 2 WH ready — LED blinking, OLED active.");

    let mut led_on = true;
    control.gpio_set(0, true).await;

    let delay = Duration::from_millis(500);

    loop {
        match select(Timer::after(delay), BUTTON_SIGNAL.wait()).await {
            Either::First(_) => {
                led_on = !led_on;
                control.gpio_set(0, led_on).await;
            }
            Either::Second(_) => {
                led_on = false;
                control.gpio_set(0, false).await;
                log_info!("LED deactivated");
            }
        }
    }
}
