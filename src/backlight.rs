//! The RGB backlight runs in its own task so effects like the rainbow keep going no matter what's
//! on screen.  Everything else just tells it what to do with [`set`].

use embassy_futures::select::{Either, select};
use embassy_rp::pwm::{PwmOutput, SetDutyCycle};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, signal::Signal};
use embassy_time::{Duration, Timer};

#[derive(Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum Mode {
    /// red, green, blue
    Solid([u8; 3]),
    /// Cycle through every fully-saturated hue.
    Rainbow,
}

static MODE: Signal<ThreadModeRawMutex, Mode> = Signal::new();

pub fn set(mode: Mode) {
    MODE.signal(mode);
}

const RAINBOW_FRAME: Duration = Duration::from_hz(60);
/// Hue is in 1/256ths of a sixth of the color wheel, so a full cycle is 6 * 256.
const HUE_CYCLE: u16 = 6 * 256;
/// Hue advanced per frame: a full cycle every 256 frames, about 4.3s.
const HUE_STEP: u16 = HUE_CYCLE / 256;

/// `pwms` is red, green, blue.
#[embassy_executor::task]
pub async fn run(mut pwms: [PwmOutput<'static>; 3]) -> ! {
    let mut mode = Mode::Solid([0; 3]);
    let mut hue = 0;

    loop {
        match mode {
            Mode::Solid(rgb) => {
                show(&mut pwms, rgb);
                mode = MODE.wait().await;
            }
            Mode::Rainbow => {
                show(&mut pwms, rainbow(hue));
                hue = (hue + HUE_STEP) % HUE_CYCLE;
                if let Either::First(m) = select(MODE.wait(), Timer::after(RAINBOW_FRAME)).await {
                    mode = m;
                }
            }
        }
    }
}

fn show(pwms: &mut [PwmOutput<'static>; 3], rgb: [u8; 3]) {
    for (pwm, value) in pwms.iter_mut().zip(rgb) {
        pwm.set_duty_cycle(value.into()).unwrap();
    }
}

/// The fully-saturated, full-brightness color at `hue` (out of [`HUE_CYCLE`]), per the "HSV to
/// RGB" section of Wikipedia's HSL and HSV article.
fn rainbow(hue: u16) -> [u8; 3] {
    let rising = (hue % 256) as u8;
    let falling = 255 - rising;
    match hue / 256 {
        0 => [255, rising, 0],
        1 => [falling, 255, 0],
        2 => [0, 255, rising],
        3 => [0, falling, 255],
        4 => [rising, 0, 255],
        _ => [255, 0, falling],
    }
}
