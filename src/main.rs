//! Blinks the LED and displays text on an SSD1306 OLED screen via I2C.
#![no_std]
#![no_main]

use bsp::entry;
use defmt::*;
use defmt_rtt as _;
use embedded_hal::digital::OutputPin;
use panic_probe as _;

// Import embedded-graphics and SSD1306 drivers
use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    text::Text,
};
use ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306};

use bsp::hal::{
    clocks::{init_clocks_and_plls, Clock},
    fugit::RateExtU32,
    gpio::{FunctionI2C, Pin, PullUp},
    pac,
    sio::Sio,
    watchdog::Watchdog,
    I2C,
};
use rp_pico as bsp;

#[entry]
fn main() -> ! {
    info!("Program start");
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let sio = Sio::new(pac.SIO);

    let external_xtal_freq_hz = 12_000_000u32;
    let clocks = init_clocks_and_plls(
        external_xtal_freq_hz,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

    let pins = bsp::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // On-board LED
    let mut led_pin = pins.led.into_push_pull_output();

    // Configure GP4 (SDA) and GP5 (SCL) for hardware I2C0 with pull-up enabled
    let sda_pin: Pin<_, FunctionI2C, PullUp> = pins.gpio4.reconfigure();
    let scl_pin: Pin<_, FunctionI2C, PullUp> = pins.gpio5.reconfigure();

    let i2c = I2C::i2c0(
        pac.I2C0,
        sda_pin,
        scl_pin,
        400.kHz(),
        &mut pac.RESETS,
        &clocks.system_clock,
    );

    // Set up SSD1306 display
    let interface = I2CDisplayInterface::new(i2c);
    let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();

    if display.init().is_err() {
        error!("Display initialization failed!");
    } else {
        info!("Display initialized successfully.");
    }

    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X10)
        .text_color(BinaryColor::On)
        .build();

    // Render initial static UI
    Text::new("PICO BTC SIGNER", Point::new(10, 15), text_style)
        .draw(&mut display)
        .unwrap();

    Text::new("Status: Ready", Point::new(10, 35), text_style)
        .draw(&mut display)
        .unwrap();

    display.flush().unwrap();

    loop {
        info!("on!");
        led_pin.set_high().unwrap();
        delay.delay_ms(1000);

        info!("off!");
        led_pin.set_low().unwrap();
        delay.delay_ms(1000);
    }
}
