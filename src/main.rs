//! Template

#![no_std]
#![no_main]

mod hardware;

use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::{
    clocks::ClockConfig,
    gpio::{self, Output},
    pwm::{self, Pwm},
};
use embassy_time::{Duration, Timer};
use embedded_graphics::{
    Drawable,
    geometry::{Point, Size},
    mono_font,
    pixelcolor::BinaryColor,
    primitives::{PrimitiveStyleBuilder, Rectangle, StyledDrawable},
    text::{self, Text},
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
        display(hardware::init_display(
            peripherals.SPI0,
            peripherals.PIN_1,
            peripherals.PIN_3,
            peripherals.PIN_2,
            peripherals.PIN_5,
            peripherals.PIN_7,
        ))
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
async fn display(mut display: hardware::DrawTarget) {
    let character_style = mono_font::MonoTextStyleBuilder::new()
        .font(&embedded_vintage_fonts::FONT_8X16)
        .background_color(BinaryColor::Off)
        .text_color(BinaryColor::On)
        .build();
    Text::with_baseline(
        "C:\\> cd spot\nC:\\spot> run",
        Point { x: 0, y: 0 },
        character_style,
        text::Baseline::Top,
    )
    .draw(&mut display);

    let box_style = PrimitiveStyleBuilder::new()
        .stroke_color(BinaryColor::On)
        .stroke_width(2)
        .build();
    Rectangle::new(
        Point { x: 35, y: 2 },
        Size {
            width: 12,
            height: 7,
        },
    )
    .draw_styled(&box_style, &mut display);

    display.flush().unwrap();

    display.set_display_on(true).unwrap();

    core::future::pending::<()>().await;
}
