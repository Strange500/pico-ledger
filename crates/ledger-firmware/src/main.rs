#![no_std]
#![no_main]

extern crate alloc;

use cyw43_pio::{PioSpi, DEFAULT_CLOCK_DIVIDER};
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::config::Config;
use embassy_rp::dma;
use embassy_rp::gpio::{Input, Level, Output, Pull};
use embassy_rp::i2c::{Config as I2cConfig, I2c, InterruptHandler as I2cInterruptHandler};
use embassy_rp::peripherals::{DMA_CH0, I2C0, PIO0, TRNG, USB};
use embassy_rp::pio::{InterruptHandler as PioInterruptHandler, Pio};
use embassy_rp::trng::{Config as TrngConfig, InterruptHandler as TrngInterruptHandler, Trng};
use embassy_rp::usb::{Driver, InterruptHandler as UsbInterruptHandler};
use embassy_time::{Duration, Timer};
use embedded_alloc::LlffHeap as Heap;
use panic_probe as _;
use static_cell::StaticCell;

use crate::tasks::button::dual_button_task;
use crate::tasks::display::display_task;
use crate::tasks::lock::lock_task;
use crate::tasks::signer::signer_task;
use crate::tasks::usb::logger_task;
use crate::tasks::wifi::cyw43_task;

#[macro_use]
mod macros;
mod signals;
mod tasks;

#[global_allocator]
static HEAP: Heap = Heap::empty();

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => PioInterruptHandler<PIO0>;
    DMA_IRQ_0 => dma::InterruptHandler<DMA_CH0>;
    USBCTRL_IRQ => UsbInterruptHandler<USB>;
    TRNG_IRQ => TrngInterruptHandler<TRNG>;
    I2C0_IRQ => I2cInterruptHandler<I2C0>;
});

static FW: cyw43::Aligned<cyw43::A4, [u8; 231_077]> =
    cyw43::Aligned(*cyw43_firmware::CYW43_43439A0);
static CLM: cyw43::Aligned<cyw43::A4, [u8; 984]> =
    cyw43::Aligned(*cyw43_firmware::CYW43_43439A0_CLM);
static NVRAM: cyw43::Aligned<cyw43::A4, [u8; 742]> =
    cyw43::Aligned(*include_bytes!("../firmware/nvram_rp2040.bin"));
static CYW43_STATE: StaticCell<cyw43::State> = StaticCell::new();

macro_rules! spawn_task {
    ($spawner:expr, $task_init:expr, $name:expr) => {
        match $task_init {
            Ok(task) => $spawner.spawn(task),
            Err(_) => log_error!("Failed to spawn {}", $name),
        }
    };
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    #[allow(unsafe_code)]
    unsafe {
        embedded_alloc::init!(HEAP, 128 * 1024);
    }

    let p = embassy_rp::init(Config::default());

    // --- 1. USB CDC-ACM ---
    let usb_driver = Driver::new(p.USB, Irqs);
    spawn_task!(spawner, logger_task(usb_driver), "USB Logger");
    log_info!("RP2350 (Pico 2 WH) booted - USB serial active");

    // --- 2. Générateur Aléatoire (TRNG) ---
    let trng = Trng::new(p.TRNG, Irqs, TrngConfig::default());
    spawn_task!(spawner, signer_task(trng), "Signer Task");

    // --- 3. Écran OLED ---
    let mut i2c_config = I2cConfig::default();
    i2c_config.frequency = 400_000;
    let i2c = I2c::new_async(p.I2C0, p.PIN_5, p.PIN_4, Irqs, i2c_config);
    spawn_task!(spawner, display_task(i2c), "Display Task");

    // --- 4. Boutons Physiques ---
    let btn_next = Input::new(p.PIN_2, Pull::Up);
    let btn_back = Input::new(p.PIN_3, Pull::Up);
    spawn_task!(spawner, dual_button_task(btn_back, btn_next), "Button Task");

    // --- 5. Coprocesseur Wi-Fi (CYW43) ---
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

    let state = CYW43_STATE.init(cyw43::State::new());
    let (_net_device, mut control, runner) = cyw43::new(state, pwr, spi, &FW, &NVRAM).await;

    spawn_task!(spawner, cyw43_task(runner), "WiFi Task");
    control.init(&*CLM).await;
    control
        .set_power_management(cyw43::PowerManagementMode::None)
        .await;

    // --- 6. Verrouillage / Sécurité ---
    spawn_task!(spawner, lock_task(), "Lock Task");

    log_info!("Pico 2 WH ready — LED blinking, OLED active.");

    // --- Boucle Principale de Statut ---
    let mut led_on = true;
    control.gpio_set(0, true).await;
    let delay = Duration::from_millis(500);

    loop {
        led_on = !led_on;
        control.gpio_set(0, led_on).await;
        Timer::after(delay).await;
    }
}
