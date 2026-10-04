use embassy_time::Duration;
use embedded_graphics::{
    Drawable,
    geometry::Point,
    mono_font::{MonoFont, MonoTextStyle},
    pixelcolor::BinaryColor,
    text::Text,
};

use crate::hardware;
use crate::ui::{self, Button, Canvas, Response, Screen, text_input::TextInput};

const BIG_FONT: &MonoFont = &embedded_vintage_fonts::FONT_24X32;
const BIG_TEXT: MonoTextStyle<'static, BinaryColor> = MonoTextStyle::new(BIG_FONT, BinaryColor::On);
const BIG_CHAR_WIDTH: u32 = BIG_FONT.character_size.width;

/// Scroll speeds Up/Down step through, slowest first, as (time per tick, pixels per tick).  Slow
/// speeds tick less often rather than moving less than a pixel; fast ones move more per tick
/// rather than redrawing more often than the LCD can keep up with.
const SCROLL_SPEEDS: [(Duration, u32); 8] = [
    (Duration::from_millis(150), 1),
    (Duration::from_millis(100), 1),
    (Duration::from_millis(75), 1),
    (Duration::from_millis(75), 2),
    (Duration::from_millis(50), 2),
    (Duration::from_millis(50), 3),
    (Duration::from_millis(40), 4),
    (Duration::from_millis(33), 6),
];
const DEFAULT_SCROLL_SPEED: usize = 3;
/// blank space between the end of the text and its next repetition
const SCROLL_GAP: u32 = 2 * BIG_CHAR_WIDTH;

/// Shows some text as big as the screen allows, scrolling it if it doesn't fit; Center/Right edits
/// it, and Up/Down make the scrolling faster/slower.
pub struct TextDisplay {
    input: TextInput<64>,
    editing: bool,
    /// how far the text has scrolled left, in pixels
    scroll: u32,
    /// index into `SCROLL_SPEEDS`
    speed: usize,
}

impl TextDisplay {
    pub fn new() -> Self {
        Self {
            input: TextInput::new(">", ""),
            editing: false,
            scroll: 0,
            speed: DEFAULT_SCROLL_SPEED,
        }
    }

    fn text_width(&self) -> u32 {
        self.input.value().len() as u32 * BIG_CHAR_WIDTH
    }

    fn scrolls(&self) -> bool {
        !self.editing && self.text_width() > hardware::DISPLAY_WIDTH
    }
}

/// Draw `text` in the big font with its left edge at `x`, skipping characters that are off screen.
fn draw_big(canvas: &mut Canvas, text: &str, x: i32) {
    let char_width = BIG_CHAR_WIDTH as i32;
    let first = ((-x).max(0) / char_width) as usize;
    let last = ((hardware::DISPLAY_WIDTH as i32 - x + char_width - 1) / char_width)
        .clamp(0, text.len() as i32) as usize;
    if first >= last {
        return;
    }
    // text from a TextInput is ASCII, so slicing by byte is slicing by character
    Text::with_text_style(
        &text[first..last],
        Point::new(x + first as i32 * char_width, 0),
        BIG_TEXT,
        ui::TOP_LEFT,
    )
    .draw(canvas)
    .unwrap();
}

impl Screen for TextDisplay {
    fn enter(&mut self) {
        self.editing = false;
        self.scroll = 0;
    }

    fn draw(&self, canvas: &mut Canvas) {
        if self.editing {
            return self.input.draw(canvas);
        }

        let text = self.input.value();
        if text.is_empty() {
            Text::with_text_style(
                "o to edit, < to exit",
                Point::zero(),
                ui::TEXT,
                ui::TOP_LEFT,
            )
            .draw(canvas)
            .unwrap();
        } else if self.scrolls() {
            let x = -(self.scroll as i32);
            draw_big(canvas, text, x);
            draw_big(canvas, text, x + (self.text_width() + SCROLL_GAP) as i32);
        } else {
            draw_big(
                canvas,
                text,
                ((hardware::DISPLAY_WIDTH - self.text_width()) / 2) as i32,
            );
        }
    }

    fn handle(&mut self, button: Button) -> Response {
        if self.editing {
            if self.input.handle(button) == Response::Exit {
                self.editing = false;
                self.scroll = 0;
                if self.input.accepted() {
                    defmt::info!("text set to {}", self.input.value());
                }
            }
            return Response::Redraw;
        }

        match button {
            Button::Center | Button::Right => {
                self.editing = true;
                self.input.enter();
                Response::Redraw
            }
            Button::Left => Response::Exit,
            Button::Up => {
                self.speed = (self.speed + 1).min(SCROLL_SPEEDS.len() - 1);
                Response::Ignored
            }
            Button::Down => {
                self.speed = self.speed.saturating_sub(1);
                Response::Ignored
            }
        }
    }

    fn tick_interval(&self) -> Option<Duration> {
        self.scrolls().then_some(SCROLL_SPEEDS[self.speed].0)
    }

    fn tick(&mut self) -> Response {
        let (_, step) = SCROLL_SPEEDS[self.speed];
        self.scroll = (self.scroll + step) % (self.text_width() + SCROLL_GAP);
        Response::Redraw
    }
}
