use embassy_rp::{
    Peri,
    gpio::{self, Output},
    peripherals,
    spi::{self, Spi},
};
use st7565::{GraphicsPageBuffer, ST7565};
use static_cell::StaticCell;

pub type DisplayCS = peripherals::PIN_1;
pub type DisplayTX = peripherals::PIN_3;
pub type DisplayClk = peripherals::PIN_2;
pub type DisplayA0 = peripherals::PIN_5;
pub type DisplayReset = peripherals::PIN_7;
pub type DisplaySPI = peripherals::SPI0;

pub struct BadgeDisplay;

impl st7565::DisplaySpecs<128, 32, 4> for BadgeDisplay {
    const FLIP_ROWS: bool = true;

    const FLIP_COLUMNS: bool = false;

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

    const COLUMN_OFFSET: u8 = 0;
}

// This type is a nested nightmare; its real type was carefully obtained by ... temporarily using
// `()` and then copy/pasting out of the error message.  You might need to do that again if
// something changes about initialization.
pub type DrawTarget = ST7565<
    display_interface_spi::SPIInterface<
        embedded_hal_bus::spi::ExclusiveDevice<
            embassy_rp::spi::Spi<'static, DisplaySPI, spi::Blocking>,
            Output<'static>,
            embedded_hal_bus::spi::NoDelay,
        >,
        Output<'static>,
    >,
    BadgeDisplay,
    st7565::modes::GraphicsMode<'static, 128, 4>,
    128,
    32,
    4,
>;

pub fn init_display(
    spi: Peri<'static, DisplaySPI>,
    cs: Peri<'static, DisplayCS>,
    tx: Peri<'static, DisplayTX>,
    clk: Peri<'static, DisplayClk>,
    a0: Peri<'static, DisplayA0>,
    rst: Peri<'static, DisplayReset>,
) -> DrawTarget {
    let cs = Output::new(cs, gpio::Level::Low);
    let a0 = Output::new(a0, gpio::Level::Low);
    let mut rst = Output::new(rst, gpio::Level::Low);

    let spi = Spi::new_blocking_txonly(spi, clk, tx, {
        let mut config = spi::Config::default();
        config.frequency = 500_000;
        config
    });

    let spi = embedded_hal_bus::spi::ExclusiveDevice::new_no_delay(spi, cs).unwrap();
    let spi = display_interface_spi::SPIInterface::new(spi, a0);
    let mut display = st7565::ST7565::new(spi, BadgeDisplay);

    display.reset(&mut rst, &mut embassy_time::Delay).unwrap();
    // don't run the Drop implementation on the Output, which (via `impl Drop for Flex`) resets the
    // GPIO pin to high-Z, which will in turn accidentally hold the display in reset.
    core::mem::forget(rst);

    let buffer = {
        static BUFFER: StaticCell<GraphicsPageBuffer<128, 4>> = StaticCell::new();
        // PANIC SAFETY: this function takes specific pins by move, so it's impossible to call
        // init_display() twice.
        BUFFER.init_with(GraphicsPageBuffer::new)
    };
    return display.into_graphics_mode(buffer);
}
