#![no_std]
#![no_main]

use core::cell::Cell;

use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_rp::gpio::{Input, Level, Output, Pull};
use embassy_rp::i2c::{Config as I2cConfig, I2c};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::blocking_mutex::Mutex;
use embassy_time::Timer;
use heapless::String;
use panic_halt as _;

use core::fmt::Write;
use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    
    text::Text,
};
use ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306};
static CLICK_COUNT: Mutex<CriticalSectionRawMutex, Cell<u32>> = Mutex::new(Cell::new(0));

#[embassy_executor::task]
async fn button_task(mut button: Input<'static>, mut led: Output<'static>) {
    loop {
        // Suspends until GP2 generates a falling edge (button press)
        button.wait_for_falling_edge().await;

        led.toggle();
        CLICK_COUNT.lock(|count| {
            count.set(count.get() + 1);
        });
        // Non-blocking debounce delay
        Timer::after_millis(50).await;
    }
}
#[embassy_executor::main]
async fn main(spawner: Spawner) {
    // 1. Initialize chip clocks, PLLs, and watchdog in one call
    let p = embassy_rp::init(Default::default());

    // 2. Configure Button on GP2 with internal pull-up
    let mut button = Input::new(p.PIN_2, Pull::Up);

    // 3. Configure I2C0 for SSD1306 (GP4 = SDA, GP5 = SCL)
    let i2c = I2c::new_blocking(p.I2C0, p.PIN_5, p.PIN_4, I2cConfig::default());
    let interface = I2CDisplayInterface::new(i2c);
    let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();
    display.init().unwrap();

    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X10)
        .text_color(BinaryColor::On)
        .build();

    // Initial render
    display.clear_buffer();
    Text::new("Ledger Ready", Point::new(10, 20), text_style)
        .draw(&mut display)
        .unwrap();
    display.flush().unwrap();

    let led = Output::new(p.PIN_25, Level::Low);

    spawner.spawn(button_task(button, led).unwrap());

    // 4. Async Event Loop
    loop {
        // Read the atomic counter
        let count = CLICK_COUNT.lock(|click| click.get());

        // Format into a fixed-size stack buffer
        let mut text: String<32> = String::new();
        write!(&mut text, "Click Count: {}", count).unwrap();

        // Update the screen
        display.clear_buffer();
        Text::new(&text, Point::new(10, 20), text_style)
            .draw(&mut display)
            .unwrap();
        display.flush().unwrap();

        // Screen refresh interval
        Timer::after_millis(200).await;
    }
}
