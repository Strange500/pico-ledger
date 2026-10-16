//! Blinks the LED and displays text on an SSD1306 OLED screen via I2C.
#![no_std]
#![no_main]

use bsp::entry;
use defmt::*;
use defmt_rtt as _;
use embedded_hal::digital::{InputPin, OutputPin};
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
    gpio::FunctionI2C,
    pac,
    sio::Sio,
    watchdog::Watchdog,
    I2C,
};
use rp_pico as bsp;
use rp_pico::hal;
#[entry]
fn main() -> ! {
    info!("Program start");
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();
    let mut watchdog = Watchdog::new(pac.WATCHDOG);

    let clocks = hal::clocks::init_clocks_and_plls(
        rp_pico::XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    // The delay object lets us wait for specified amounts of time (in
    // milliseconds)
    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

    // The single-cycle I/O block controls our GPIO pins
    let sio = hal::Sio::new(pac.SIO);
    let pins = rp_pico::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let mut led_pin = pins.led.into_push_pull_output();
    let mut btn = pins.gpio2.into_pull_up_input();
    let mut toggle_status = false;
    let mut was_pressed = false;
    loop {
        let is_pressed = btn.is_low().unwrap();
        if is_pressed && !was_pressed {
            toggle_status = !toggle_status;
        }
        if toggle_status {
            led_pin.set_high().unwrap();
        } else {
            led_pin.set_low().unwrap();
        }
        delay.delay_ms(50u32); // Debounce delay
        was_pressed = is_pressed;
    }
}
