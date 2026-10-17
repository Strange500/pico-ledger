#![no_std]
#![no_main]

use cyw43_pio::{PioSpi, DEFAULT_CLOCK_DIVIDER};
use defmt::info;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{DMA_CH0, PIO0};
use embassy_rp::pio::{InterruptHandler, Pio};
use embassy_rp::{bind_interrupts, dma};
use embassy_time::{Duration, Timer};
use panic_halt as _;
use static_cell::StaticCell;

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
    DMA_IRQ_0 => dma::InterruptHandler<DMA_CH0>;
});

#[embassy_executor::task]
async fn cyw43_task(
    runner: cyw43::Runner<'static, cyw43::SpiBus<Output<'static>, PioSpi<'static, PIO0, 0>>>,
) -> ! {
    runner.run().await
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

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
    let (_device, mut control, runner) = cyw43::new(state, pwr, spi, &FW, &NVRAM).await;
    spawner.spawn(cyw43_task(runner).unwrap());
    control.init(&*CLM).await;

    info!("Pico 2 W ready");
    let delay = Duration::from_millis(500);

    loop {
        control.gpio_set(0, true).await;
        Timer::after(delay).await;
        control.gpio_set(0, false).await;
        Timer::after(delay).await;
    }
}
