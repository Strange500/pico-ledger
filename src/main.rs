#![no_std]
#![no_main]

extern crate alloc;

use cyw43_pio::{PioSpi, DEFAULT_CLOCK_DIVIDER};
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use embassy_rp::bind_interrupts;
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

#[macro_use]
mod macros;
mod signals;
mod tasks;
mod ui;
mod wordlist;

#[global_allocator]
static HEAP: Heap = Heap::empty();

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => PioInterruptHandler<PIO0>;
    DMA_IRQ_0 => dma::InterruptHandler<DMA_CH0>;
    USBCTRL_IRQ => UsbInterruptHandler<USB>;
    TRNG_IRQ => TrngInterruptHandler<TRNG>;
    I2C0_IRQ => I2cInterruptHandler<I2C0>;
});

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    unsafe {
        embedded_alloc::init!(HEAP, 128 * 1024);
    }

    let p = embassy_rp::init(Default::default());

    // 1. Démarrage du logger USB CDC-ACM
    let usb_driver = Driver::new(p.USB, Irqs);
    spawner.spawn(tasks::usb::logger_task(usb_driver).unwrap());

    log_info!("RP2350 (Pico 2 WH) booted - USB serial active");

    // 2. Démarrage de l'affichage OLED SSD1306 (GP4 = SDA, GP5 = SCL)
    let mut i2c_config = I2cConfig::default();
    i2c_config.frequency = 400_000;
    let i2c = I2c::new_async(p.I2C0, p.PIN_5, p.PIN_4, Irqs, i2c_config);
    spawner.spawn(tasks::display::display_task(i2c).unwrap());

    // 3. Démarrage du générateur de mots TRNG
    let trng = Trng::new(p.TRNG, Irqs, TrngConfig::default());
    spawner.spawn(tasks::rng::rd_word(trng).unwrap());

    // 4. Démarrage de la surveillance du bouton sur GP2
    let button = Input::new(p.PIN_2, Pull::Up);
    spawner.spawn(tasks::button::button_task(button).unwrap());

    // 5. Initialisation du contrôleur sans fil CYW43 (qui pilote la LED sur Pico 2 W/WH)
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
    spawner.spawn(tasks::wifi::cyw43_task(runner).unwrap());
    control.init(&*CLM).await;
    control
        .set_power_management(cyw43::PowerManagementMode::None)
        .await;

    log_info!("Pico 2 WH ready — LED blinking, OLED active.");

    let mut led_on = true;
    control.gpio_set(0, true).await;

    let delay = Duration::from_millis(500);

    // Boucle de contrôle de la LED de statut
    loop {
        led_on = !led_on;
        control.gpio_set(0, led_on).await;
        Timer::after(delay).await;
    }
}
