//! Template

#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::{
    Peri,
    clocks::ClockConfig,
    gpio::{self, Output},
    i2c::{self, Config},
    spi::{self, ClkPin, CsPin, MosiPin, Spi},
};
use embassy_time::{Duration, Timer};
use embedded_hal_1::i2c::I2c;
use rand::{RngExt, SeedableRng, distr::Uniform};
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let peripherals = embassy_rp::init(embassy_rp::config::Config::new(ClockConfig::crystal(
        12_000_000,
    )));
    lol(
        peripherals.SPI0,
        peripherals.PIN_1,
        peripherals.PIN_3,
        peripherals.PIN_2,
    );

    let mut led = gpio::Output::new(peripherals.PIN_25, gpio::Level::Low);
    info!("Hello World!");

    let mut a0 = Output::new(peripherals.PIN_6, gpio::Level::Low);
    let mut reset = Output::new(peripherals.PIN_7, gpio::Level::High);

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

#[inline(never)]
fn lol<T: spi::Instance>(
    spi: Peri<'static, T>,
    cs: Peri<'static, impl CsPin<T>>,
    tx: Peri<'static, impl MosiPin<T>>,
    clk: Peri<'static, impl ClkPin<T>>,
) {
    Spi::new_blocking_txonly(spi, clk, tx, {
        let mut config = spi::Config::default();
        config.frequency = 1_000_000;
        config
    });
}
