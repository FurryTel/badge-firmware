//! A tiny screen system: each [`Screen`] knows how to draw itself and how to react to a button
//! press.  Screens compose (see [`menu::Menu`]), so the whole UI is a single root screen that
//! [`run`] drives.  Screens that animate ask for periodic [`Screen::tick`]s.

pub mod menu;
pub mod text_input;

use embassy_futures::select::{Either, select};
use embassy_time::{Duration, Timer};
use embedded_graphics::{
    draw_target::DrawTarget as _,
    mono_font::{self, MonoTextStyle, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    text::{self, TextStyle, TextStyleBuilder},
};

use crate::hardware::Buttons;

/// What screens draw on.  This is the concrete display type rather than a generic
/// `embedded_graphics::DrawTarget`, which keeps [`Screen`] usable as a trait object.
pub type Canvas = crate::hardware::DrawTarget;

#[derive(Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum Button {
    Up,
    Down,
    Left,
    Right,
    Center,
}

#[derive(Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum Response {
    /// The event changed something; the screen needs to be redrawn.
    Redraw,
    /// The event didn't do anything.
    Ignored,
    /// The screen is done; whoever is showing it should take back over.
    Exit,
}

pub trait Screen {
    /// Called each time the screen is opened, before it's first drawn.  Not called on redraws.
    fn enter(&mut self) {}

    /// Draw the whole screen.  The canvas has already been cleared.
    fn draw(&self, canvas: &mut Canvas);

    fn handle(&mut self, button: Button) -> Response;

    /// How often the screen wants [`Screen::tick`] called, or `None` (the default) for never.
    /// Checked after every event, so a screen can start and stop animating as it likes.
    fn tick_interval(&self) -> Option<Duration> {
        None
    }

    /// Called every [`Screen::tick_interval`] while that's `Some`.
    fn tick(&mut self) -> Response {
        Response::Ignored
    }
}

/// Drive `root` forever: wait for a button (or a tick, if the screen wants them), hand it to the
/// screen, redraw if asked.
pub async fn run(canvas: &mut Canvas, buttons: &mut Buttons, root: &mut dyn Screen) -> ! {
    canvas.set_display_on(true).unwrap();
    root.enter();
    redraw(canvas, root);

    loop {
        let press = buttons.next_press();
        let event = match root.tick_interval() {
            Some(interval) => select(press, Timer::after(interval)).await,
            None => Either::First(press.await),
        };
        let response = match event {
            Either::First(button) => {
                defmt::debug!("button {}", button);
                root.handle(button)
            }
            Either::Second(()) => root.tick(),
        };
        match response {
            // the root has nowhere to exit to, so just show it again
            Response::Redraw | Response::Exit => redraw(canvas, root),
            Response::Ignored => {}
        }
    }
}

fn redraw(canvas: &mut Canvas, screen: &dyn Screen) {
    canvas.clear(BinaryColor::Off).unwrap();
    screen.draw(canvas);
    canvas.flush().unwrap();
}

pub const FONT: &mono_font::MonoFont = &mono_font::ascii::FONT_6X10;

/// Light text on a dark background.
pub const TEXT: MonoTextStyle<'static, BinaryColor> = MonoTextStyle::new(FONT, BinaryColor::On);
/// Dark text, for drawing on top of a highlight.
pub const TEXT_INVERTED: MonoTextStyle<'static, BinaryColor> =
    MonoTextStyle::new(FONT, BinaryColor::Off);
/// Dark text with its own light background, for highlighting a bit of text.
pub const TEXT_HIGHLIGHTED: MonoTextStyle<'static, BinaryColor> = MonoTextStyleBuilder::new()
    .font(FONT)
    .text_color(BinaryColor::Off)
    .background_color(BinaryColor::On)
    .build();

/// For when [`FONT`] is too big, e.g. the on-screen keyboard.
/// ISO 8859-1 rather than ASCII for a few extra symbols, like the keyboard's « and ».
pub const SMALL_FONT: &mono_font::MonoFont = &mono_font::iso_8859_1::FONT_4X6;
pub const SMALL_TEXT: MonoTextStyle<'static, BinaryColor> =
    MonoTextStyle::new(SMALL_FONT, BinaryColor::On);
pub const SMALL_TEXT_INVERTED: MonoTextStyle<'static, BinaryColor> =
    MonoTextStyle::new(SMALL_FONT, BinaryColor::Off);

/// Position text by its top-left corner.
pub const TOP_LEFT: TextStyle = TextStyleBuilder::new()
    .alignment(text::Alignment::Left)
    .baseline(text::Baseline::Top)
    .build();
