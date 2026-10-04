//! The three discrete LEDs run in their own task so effects like random blinking keep going no
//! matter what's on screen.  Everything else just tells it what to do with [`set`].

use embassy_futures::select::{Either, select};
use embassy_rp::pwm::{PwmOutput, SetDutyCycle};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, signal::Signal};
use embassy_time::{Duration, Instant, Timer};
use rand::{RngExt, SeedableRng, distr::Uniform};

#[derive(Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum Mode {
    /// top, middle, bottom
    Solid([u8; 3]),
    /// Blink each LED on and off at random, like blinkenlights.
    Random,
}

static MODE: Signal<ThreadModeRawMutex, Mode> = Signal::new();

pub fn set(mode: Mode) {
    MODE.signal(mode);
}

/// Duty cycle of a lit LED while blinking.
const BLINK_BRIGHTNESS: u8 = 0xff;
/// Range of how long, in milliseconds, an LED stays on or off before flipping.
const BLINK_MIN_MS: u64 = 30;
const BLINK_MAX_MS: u64 = 400;

/// `pwms` is top, middle, bottom.
#[embassy_executor::task]
pub async fn run(mut pwms: [PwmOutput<'static>; 3]) -> ! {
    let mut rng = rand::rngs::SmallRng::from_seed([23; _]);
    let durations = Uniform::new_inclusive(BLINK_MIN_MS, BLINK_MAX_MS).expect("low <= high");

    let mut mode = Mode::Solid([0; 3]);
    let mut lit = [false; 3];
    let mut flip_at = [Instant::now(); 3];

    loop {
        match mode {
            Mode::Solid(values) => {
                show(&mut pwms, values);
                mode = MODE.wait().await;
            }
            Mode::Random => {
                let now = Instant::now();
                for (lit, flip_at) in lit.iter_mut().zip(&mut flip_at) {
                    if *flip_at <= now {
                        *lit = !*lit;
                        *flip_at = now + Duration::from_millis(rng.sample(durations));
                    }
                }
                show(&mut pwms, lit.map(|l| if l { BLINK_BRIGHTNESS } else { 0 }));

                // PANIC SAFETY: there are always three LEDs
                let next = *flip_at.iter().min().unwrap();
                if let Either::First(m) = select(MODE.wait(), Timer::at(next)).await {
                    mode = m;
                }
            }
        }
    }
}

fn show(pwms: &mut [PwmOutput<'static>; 3], values: [u8; 3]) {
    for (pwm, value) in pwms.iter_mut().zip(values) {
        pwm.set_duty_cycle(value.into()).unwrap();
    }
}
