use buoyant::render_target::RenderTarget;
use buoyant::{
    app::App, primitives::Size, render_target::EmbeddedGraphicsRenderTarget, view::prelude::*,
};
use embassy_rp::i2c::I2c;
use embassy_rp::peripherals::I2C0;
use embassy_time::{Duration, Timer};
use embedded_graphics::{pixelcolor::BinaryColor, prelude::*};
use ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306};

use crate::signals::MESSAGE_QUEUE;
use crate::ui::state::AppModel;

/// Background task: drive the SSD1306 128x64 OLED over I2C (GP4 SDA, GP5 SCL).
#[embassy_executor::task]
pub async fn display_task(i2c: I2c<'static, I2C0, embassy_rp::i2c::Async>) {
    let interface = I2CDisplayInterface::new(i2c);
    let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();
    let size = Size::new(display.size().width, display.size().height);
    let mut app = App::new(AppModel::Init, size, view);

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

    let mut target = EmbeddedGraphicsRenderTarget::new(&mut display);

    loop {
        if app.should_redraw() || target.clear_animation_status() {
            target.clear(BinaryColor::Off);
            // Render animated transition between source and target trees
            app.render_animated(&mut target, &BinaryColor::On);
            // Clear for the next frame
            target
                .display_mut()
                .flush()
                .expect("Failed to flush display buffer");
        }
        let msg = MESSAGE_QUEUE.receive().await;
        app.state_mut().update(msg);
    }
}

pub fn view(state: &AppModel) -> impl View<BinaryColor, AppModel> + use<> {
    state.view()
}
