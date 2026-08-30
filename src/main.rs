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
use embassy_time::{Delay, Duration, Timer};
use embedded_hal_1::i2c::I2c;
use rand::{RngExt, SeedableRng, distr::Uniform};
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
    Timer::after_millis(100).await;
    reset.set_high();

    info!("Done with reset.");

    lol(
        peripherals.SPI0,
        peripherals.PIN_1,
        peripherals.PIN_3,
        peripherals.PIN_2,
        a0,
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

#[inline(never)]
async fn lol<T: spi::Instance>(
    spi: Peri<'static, T>,
    cs: Peri<'static, impl CsPin<T>>,
    tx: Peri<'static, impl MosiPin<T>>,
    clk: Peri<'static, impl ClkPin<T>>,
    mut a0: Output<'static>,
) {
    let mut cs = Output::new(cs, gpio::Level::Low);

    let mut spi = Spi::new_blocking_txonly(spi, clk, tx, {
        let mut config = spi::Config::default();
        config.frequency = 500_000;
        config
    });

    spi.blocking_write(&[
        // taken directly from init_LCD() in the datasheet
        // except                     vreg      contrast
        //                             v           v
        0xA0, 0xAE, 0xC0, 0xA2, 0x2F, 0x21, 0x81, 0x20, 0xAF,
    ])
    .unwrap();

    for row in 0..4 {
        a0.set_low();
        spi.blocking_write(&[0xB0 + row, 0x10, 0x00]).unwrap();

        a0.set_high();
        for _two_columns in 0..64 {
            spi.blocking_write(&[0x55 ^ row]).unwrap();
            spi.blocking_write(&[0xAA ^ row]).unwrap();
        }
    }
}
