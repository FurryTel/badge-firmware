//! Template

#![no_std]
#![no_main]

use cortex_m::asm::wfi;
use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::{
    Peri,
    clocks::ClockConfig,
    gpio::{self, Output},
    i2c::{self, Config},
    spi::{self, ClkPin, CsPin, MosiPin, Spi},
};
use embassy_time::{Delay, Duration, Timer};
use embedded_graphics::{
    Drawable,
    geometry::Point,
    mono_font,
    text::{self, Text, renderer},
};
use embedded_hal_1::i2c::I2c;
use rand::{RngExt, SeedableRng, distr::Uniform};
use st7565::GraphicsPageBuffer;
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let peripherals = embassy_rp::init(embassy_rp::config::Config::new(ClockConfig::crystal(
        12_000_000,
    )));

    let mut led = gpio::Output::new(peripherals.PIN_25, gpio::Level::Low);
    info!("Hello World!");
    let a0 = Output::new(peripherals.PIN_6, gpio::Level::Low);

    let mut reset = Output::new(peripherals.PIN_7, gpio::Level::Low);

    info!("Done with reset.");

    lol(
        peripherals.SPI0,
        peripherals.PIN_1,
        peripherals.PIN_3,
        peripherals.PIN_2,
        a0,
        &mut reset,
    )
    .await;

    let mut rng = rand::rngs::SmallRng::from_seed([42; _]);
    let distribution = Uniform::new(1, 500).expect("low < high");
    let mut wait = || {
        let ms = rng.sample(distribution);
        async move {
            Timer::after(Duration::from_millis(ms)).await;
        }
    };

    loop {
        wait().await;
        led.set_high();
        wait().await;
        led.set_low();
    }
}

struct BadgeDisplay;

impl st7565::DisplaySpecs<128, 32, 4> for BadgeDisplay {
    const FLIP_ROWS: bool = false;

    const FLIP_COLUMNS: bool = true;

    const INVERTED: bool = false;

    const BIAS_MODE_1: bool = false;

    const POWER_CONTROL: st7565::types::PowerControlMode = st7565::types::PowerControlMode {
        booster_circuit: true,
        voltage_regulator_circuit: true,
        voltage_follower_circuit: true,
    };

    const VOLTAGE_REGULATOR_RESISTOR_RATIO: u8 = 1;

    const ELECTRONIC_VOLUME: u8 = 0x20;

    const BOOSTER_RATIO: st7565::types::BoosterRatio = st7565::types::BoosterRatio::StepUp2x3x4x;

    const COLUMN_OFFSET: u8 = 4;
}

#[inline(never)]
async fn lol<T: spi::Instance>(
    spi: Peri<'static, T>,
    cs: Peri<'static, impl CsPin<T>>,
    tx: Peri<'static, impl MosiPin<T>>,
    clk: Peri<'static, impl ClkPin<T>>,
    a0: Output<'static>,
    rst: &mut Output<'static>,
) {
    let mut cs = Output::new(cs, gpio::Level::Low);

    let spi = Spi::new_blocking_txonly(spi, clk, tx, {
        let mut config = spi::Config::default();
        config.frequency = 500_000;
        config
    });

    let spi = embedded_hal_bus::spi::ExclusiveDevice::new_no_delay(spi, cs).unwrap();
    let spi = display_interface_spi::SPIInterface::new(spi, a0);
    let mut display = st7565::ST7565::new(spi, BadgeDisplay);

    display.reset(rst, &mut embassy_time::Delay).unwrap();
    let mut buffer = GraphicsPageBuffer::new();
    let mut display = display.into_graphics_mode(&mut buffer);

    let character_style = mono_font::MonoTextStyleBuilder::new()
        .font(&embedded_vintage_fonts::FONT_8X16)
        .background_color(embedded_graphics::pixelcolor::BinaryColor::Off)
        .text_color(embedded_graphics::pixelcolor::BinaryColor::On)
        .build();
    Text::with_baseline(
        "C:\\> cd spot\nC:\\spot> run",
        Point { x: 0, y: 0 },
        character_style,
        text::Baseline::Top,
    )
    .draw(&mut display);
    display.flush().unwrap();

    display.set_display_on(true).unwrap();
}
