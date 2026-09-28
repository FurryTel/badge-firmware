//! Template

#![no_std]
#![no_main]

mod debounce;
mod hardware;

use core::fmt::Write as _;

use crate::hardware::Buttons;
use defmt::*;
use embassy_executor::Spawner;
use embassy_futures::select::{Either5, select5};
use embassy_rp::{
    clocks::ClockConfig,
    gpio::{self, Output},
    pwm::{self, Pwm},
};
use embassy_time::{Duration, Timer};
use embedded_graphics::{
    Drawable,
    geometry::Point,
    mono_font,
    pixelcolor::BinaryColor,
    text::{self, Text, TextStyleBuilder},
};
use rand::{RngExt, SeedableRng, distr::Uniform};

use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let peripherals = embassy_rp::init(embassy_rp::config::Config::new(ClockConfig::crystal(
        12_000_000,
    )));

    spawner.spawn(led(Output::new(peripherals.PIN_25, gpio::Level::Low)).unwrap());
    info!("Hello World!");

    spawner.spawn(
        display(
            hardware::init_display(
                peripherals.SPI0,
                peripherals.PIN_1,
                peripherals.PIN_3,
                peripherals.PIN_2,
                peripherals.PIN_5,
                peripherals.PIN_7,
            ),
            Buttons::new(
                peripherals.PIN_13,
                peripherals.PIN_12,
                peripherals.PIN_11,
                peripherals.PIN_10,
                peripherals.PIN_9,
            ),
        )
        .unwrap(),
    );

    let pwm_config = {
        let mut config = pwm::Config::default();
        config.invert_a = true;
        config.enable = true;
        config.compare_a = 0;
        config.top = 256;
        config
    };
    let mut red = Pwm::new_output_a(
        peripherals.PWM_SLICE2,
        peripherals.PIN_4,
        pwm_config.clone(),
    );
    let mut green = Pwm::new_output_a(
        peripherals.PWM_SLICE3,
        peripherals.PIN_6,
        pwm_config.clone(),
    );
    let mut blue = Pwm::new_output_a(
        peripherals.PWM_SLICE4,
        peripherals.PIN_8,
        pwm_config.clone(),
    );

    let mut hue = 0.0_f32;
    let mut frame = embassy_time::Ticker::every(Duration::from_hz(60));
    loop {
        frame.next().await;

        hue += 1.0 / 256.0;
        if hue > 1.0 {
            hue -= 1.0;
        }
        defmt::assert!(0.0 <= hue && hue <= 1.0);

        // Taken from the Wikipedia article on HSL and HSV, likely with transcription errors.
        let c = 1.0_f32;
        let h_prime = hue * 6.0;
        let x = 1.0 - ((h_prime % 2.0) - 1.0).abs();
        let (r, g, b) = if h_prime < 1.0 {
            (c, x, 0.0)
        } else if h_prime < 2.0 {
            (x, c, 0.0)
        } else if h_prime < 3.0 {
            (0.0, c, x)
        } else if h_prime < 4.0 {
            (0.0, x, c)
        } else if h_prime < 5.0 {
            (x, 0.0, c)
        } else {
            (c, 0.0, x)
        };

        let mut config = pwm_config.clone();
        config.compare_a = (r * 256.0) as u16;
        red.set_config(&config);

        config.compare_a = (g * 256.0) as u16;
        green.set_config(&config);

        config.compare_a = (b * 256.0) as u16;
        blue.set_config(&config);
    }
}

#[embassy_executor::task]
async fn led(mut led: Output<'static>) {
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

#[embassy_executor::task]
async fn display(mut display: hardware::DrawTarget, mut buttons: Buttons) {
    display.set_display_on(true).unwrap();

    let character_style = mono_font::MonoTextStyleBuilder::new()
        .font(&embedded_vintage_fonts::FONT_6X8)
        .background_color(BinaryColor::Off)
        .text_color(BinaryColor::On)
        .build();
    let text_style = TextStyleBuilder::new()
        .alignment(text::Alignment::Left)
        .baseline(text::Baseline::Top)
        .build();

    let mut up = 0;
    let mut left = 0;
    let mut center = 0;
    let mut right = 0;
    let mut down = 0;

    let draw = |display: &mut hardware::DrawTarget, x, y, i| {
        let mut buffer = heapless::Vec::<u8, 5>::new();
        core::write!(&mut buffer, "{}", i).unwrap();
        Text::with_text_style(
            unsafe { str::from_utf8_unchecked(&buffer) },
            Point { x, y },
            character_style,
            text_style,
        )
        .draw(display);
    };

    loop {
        draw(&mut display, 60, 0, up);
        draw(&mut display, 0, 8, left);
        draw(&mut display, 60, 8, center);
        draw(&mut display, 100, 8, right);
        draw(&mut display, 60, 16, down);
        display.flush().unwrap();

        match select5(
            buttons.up.wait_for_pressed(),
            buttons.left.wait_for_pressed(),
            buttons.center.wait_for_pressed(),
            buttons.right.wait_for_pressed(),
            buttons.down.wait_for_pressed(),
        )
        .await
        {
            Either5::First(_) => {
                up += 1;
            }
            Either5::Second(_) => {
                left += 1;
            }
            Either5::Third(_) => {
                center += 1;
            }
            Either5::Fourth(_) => {
                right += 1;
            }
            Either5::Fifth(_) => {
                down += 1;
            }
        }
    }
}
