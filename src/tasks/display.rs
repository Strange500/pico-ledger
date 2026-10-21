use embassy_rp::i2c::I2c;
use embassy_rp::peripherals::I2C0;
use embassy_time::{Duration, Timer};
use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Line, PrimitiveStyle},
    text::Text,
};
use ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306};

use crate::signals::WORD_SIGNAL;

/// Background task: drive the SSD1306 128x64 OLED over I2C (GP4 SDA, GP5 SCL).
#[embassy_executor::task]
pub async fn display_task(i2c: I2c<'static, I2C0, embassy_rp::i2c::Async>) {
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
