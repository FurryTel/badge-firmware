//! Template

#![no_std]
#![no_main]

mod debounce;
mod hardware;

use core::fmt::Write as _;

use crate::hardware::Buttons;
use defmt::*;
use embassy_executor::Spawner;
use embassy_futures::select::{Either4, select4};
use embassy_rp::{
    clocks::ClockConfig,
    gpio::{self, Output},
    pwm::{self, Pwm, SetDutyCycle},
};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, signal::Signal};
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

trait Steppable {
    fn inc(&mut self);
    fn dec(&mut self);
}

#[derive(Default, Clone, Copy)]
struct Value<const STEP: u8> {
    value: u8,
}

impl<const STEP: u8> Steppable for Value<STEP> {
    fn inc(&mut self) {
        if self.value <= (u8::MAX - STEP) {
            self.value += STEP;
        }
    }

    fn dec(&mut self) {
        if self.value >= STEP {
            self.value -= STEP;
        }
    }
}

#[derive(Default, Clone, Copy)]
struct UiState {
    highlighted: u8,
    data: ([Value<0x11>; 3], [Value<1>; 3]),
}

impl UiState {
    fn at_highlighted_mut(&mut self) -> &mut dyn Steppable {
        defmt::assert!(self.highlighted < 6);
        if self.highlighted < 3 {
            &mut self.data.0[self.highlighted as usize]
        } else {
            &mut self.data.1[self.highlighted as usize % 3]
        }
    }
}

static UI_STATE: Signal<ThreadModeRawMutex, UiState> = Signal::new();

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

    let mut buttons = Buttons::new(
        peripherals.PIN_13,
        peripherals.PIN_12,
        peripherals.PIN_11,
        peripherals.PIN_10,
        peripherals.PIN_9,
    );

    let backlight_config = {
        let mut config = pwm::Config::default();
        config.invert_a = true;
        config.enable = true;
        config.compare_a = 0;
        config.top = 256;
        config
    };
    let mut backlight_red = Pwm::new_output_a(
        peripherals.PWM_SLICE2,
        peripherals.PIN_4,
        backlight_config.clone(),
    )
    .split()
    .0
    .unwrap();
    let mut backlight_green = Pwm::new_output_a(
        peripherals.PWM_SLICE3,
        peripherals.PIN_6,
        backlight_config.clone(),
    )
    .split()
    .0
    .unwrap();
    let mut backlight_blue = Pwm::new_output_a(
        peripherals.PWM_SLICE4,
        peripherals.PIN_8,
        backlight_config.clone(),
    )
    .split()
    .0
    .unwrap();

    let led_config = {
        let mut config = backlight_config.clone();
        config.invert_a = false;
        config
    };

    let (mut led_bottom, mut led_middle) = {
        let (a, b) = Pwm::new_output_ab(
            peripherals.PWM_SLICE0,
            peripherals.PIN_16,
            peripherals.PIN_17,
            led_config.clone(),
        )
        .split();
        (a.unwrap(), b.unwrap())
    };
    let mut led_top = {
        Pwm::new_output_a(peripherals.PWM_SLICE1, peripherals.PIN_18, led_config)
            .split()
            .0
            .unwrap()
    };

    let mut ui_state = UiState::default();
    UI_STATE.signal(ui_state);

    loop {
        match select4(
            buttons.up.wait_for_pressed(),
            buttons.down.wait_for_pressed(),
            buttons.left.wait_for_pressed(),
            buttons.right.wait_for_pressed(),
        )
        .await
        {
            Either4::First(_) => {
                ui_state.at_highlighted_mut().inc();
            }
            Either4::Second(_) => {
                ui_state.at_highlighted_mut().dec();
            }
            Either4::Third(_) => {
                ui_state.highlighted -= 1;
                ui_state.highlighted %= 6;
            }
            Either4::Fourth(_) => {
                ui_state.highlighted += 1;
                ui_state.highlighted %= 6;
            }
        }

        let ([r, g, b], [dt, dm, db]) = ui_state.data;
        backlight_red.set_duty_cycle(r.value.into()).unwrap();
        backlight_green.set_duty_cycle(g.value.into()).unwrap();
        backlight_blue.set_duty_cycle(b.value.into()).unwrap();
        led_top.set_duty_cycle(dt.value.into()).unwrap();
        led_middle.set_duty_cycle(dm.value.into()).unwrap();
        led_bottom.set_duty_cycle(db.value.into()).unwrap();

        UI_STATE.signal(ui_state);
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
    display.set_display_on(true).unwrap();

    let regular = mono_font::MonoTextStyleBuilder::new()
        .font(&embedded_graphics::mono_font::ascii::FONT_6X13)
        .background_color(BinaryColor::Off)
        .text_color(BinaryColor::On);
    let inverted = regular
        .clone()
        .background_color(BinaryColor::On)
        .text_color(BinaryColor::Off)
        .build();
    let regular = regular.build();
    let text_style = TextStyleBuilder::new()
        .alignment(text::Alignment::Left)
        .baseline(text::Baseline::Top)
        .build();

    Text::with_text_style("Backlight:", Point { x: 0, y: 0 }, regular, text_style)
        .draw(&mut display);
    Text::with_text_style("Discrete:", Point { x: 6, y: 15 }, regular, text_style)
        .draw(&mut display);
    display.flush().unwrap();

    let draw = |display: &mut hardware::DrawTarget, is_inverted: bool, x, y, i| {
        let mut buffer = heapless::Vec::<u8, 5>::new();
        core::write!(&mut buffer, "{:02x}", i).unwrap();
        Text::with_text_style(
            unsafe { str::from_utf8_unchecked(&buffer) },
            Point { x, y },
            if is_inverted { inverted } else { regular },
            text_style,
        )
        .draw(display);
    };

    loop {
        let state = UI_STATE.wait().await;

        let ([r, g, b], [dt, dm, db]) = state.data;
        draw(&mut display, state.highlighted == 0, 64, 0, r.value);
        draw(&mut display, state.highlighted == 1, 80, 0, g.value);
        draw(&mut display, state.highlighted == 2, 96, 0, b.value);
        draw(&mut display, state.highlighted == 3, 64, 15, dt.value);
        draw(&mut display, state.highlighted == 4, 80, 15, dm.value);
        draw(&mut display, state.highlighted == 5, 96, 15, db.value);
        display.flush().unwrap();
    }
}
