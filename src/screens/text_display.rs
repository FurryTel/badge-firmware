use embassy_time::Duration;
use embedded_graphics::{
    Drawable,
    geometry::Point,
    mono_font::{MonoFont, MonoTextStyle, ascii},
    pixelcolor::BinaryColor,
    text::Text,
};

use crate::hardware;
use crate::ui::{self, Button, Canvas, Response, Screen, text_input::TextInput};

/// Fonts Right cycles through, biggest first: 1, 2, 3, then 5 lines.
const FONTS: [&MonoFont; 4] = [
    &embedded_vintage_fonts::FONT_24X32,
    &embedded_vintage_fonts::FONT_12X16,
    &ascii::FONT_6X10,
    &ascii::FONT_4X6,
];

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
/// Longest text that can be entered.
const MAX_TEXT: usize = 256;

/// Blank space between the end of the text and its next repetition, in characters.
const SCROLL_GAP: u32 = 2;

/// Shows some text in one of several font sizes, scrolling it if it doesn't fit.  With a font small
/// enough for several lines, the text wraps from line to line, and scrolls like one long ribbon
/// threaded through all of them.
///
/// Center edits the text, Right picks the next font size, and Up/Down make the scrolling
/// faster/slower.
pub struct TextDisplay {
    input: TextInput<MAX_TEXT>,
    editing: bool,
    /// how far the text has scrolled left, in pixels
    scroll: u32,
    /// index into `SCROLL_SPEEDS`
    speed: usize,
    /// index into `FONTS`
    font: usize,
}

impl TextDisplay {
    pub fn new() -> Self {
        Self {
            input: TextInput::new(">", ""),
            editing: false,
            scroll: 0,
            speed: DEFAULT_SCROLL_SPEED,
            font: 0,
        }
    }

    fn layout(&self) -> Layout {
        Layout::new(FONTS[self.font])
    }

    fn text_width(&self) -> u32 {
        self.input.value().len() as u32 * self.layout().char_width
    }

    /// Length of the ribbon of text that scrolls: the text and then a gap before it repeats.
    fn scroll_period(&self) -> u32 {
        self.text_width() + SCROLL_GAP * self.layout().char_width
    }

    fn scrolls(&self) -> bool {
        let layout = self.layout();
        !self.editing && self.input.value().len() > layout.columns * layout.rows
    }

    /// Text that fits: one line is centered; more are wrapped and centered as a block.
    fn draw_still(&self, canvas: &mut Canvas, text: &str) {
        let layout = self.layout();
        if text.len() <= layout.columns {
            let x = (hardware::DISPLAY_WIDTH - self.text_width()) / 2;
            let y = (hardware::DISPLAY_HEIGHT - layout.line_height) / 2;
            return draw_clipped(canvas, &layout, text, x as i32, y as i32);
        }

        let lines = text.len().div_ceil(layout.columns);
        let top = (hardware::DISPLAY_HEIGHT - lines as u32 * layout.line_height) / 2;
        let left = (hardware::DISPLAY_WIDTH - layout.columns as u32 * layout.char_width) / 2;
        // text from a TextInput is ASCII, so chunking bytes is chunking characters
        for (i, line) in text.as_bytes().chunks(layout.columns).enumerate() {
            let y = top + i as u32 * layout.line_height;
            let line = core::str::from_utf8(line).unwrap();
            draw_clipped(canvas, &layout, line, left as i32, y as i32);
        }
    }

    /// Text that doesn't fit: a ribbon of the text repeating, threaded through every line, so what
    /// scrolls off the left of one line comes back on the right of the line above.
    fn draw_scrolling(&self, canvas: &mut Canvas, text: &str) {
        let layout = self.layout();
        let width = hardware::DISPLAY_WIDTH;
        let period = self.scroll_period();
        let top = (hardware::DISPLAY_HEIGHT - layout.rows as u32 * layout.line_height) / 2;
        for row in 0..layout.rows as u32 {
            // where along the ribbon this line starts
            let start = self.scroll + row * width;
            let y = (top + row * layout.line_height) as i32;
            // draw each repetition of the text that overlaps this line
            let mut copy = start / period;
            loop {
                let x = (copy * period) as i32 - start as i32;
                if x >= width as i32 {
                    break;
                }
                draw_clipped(canvas, &layout, text, x, y);
                copy += 1;
            }
        }
    }
}

/// How text fits on the screen in a given font.
struct Layout {
    style: MonoTextStyle<'static, BinaryColor>,
    char_width: u32,
    line_height: u32,
    /// characters per line
    columns: usize,
    rows: usize,
}

impl Layout {
    fn new(font: &'static MonoFont) -> Self {
        let char_width = font.character_size.width + font.character_spacing;
        let line_height = font.character_size.height;
        Self {
            style: MonoTextStyle::new(font, BinaryColor::On),
            char_width,
            line_height,
            columns: (hardware::DISPLAY_WIDTH / char_width) as usize,
            rows: (hardware::DISPLAY_HEIGHT / line_height) as usize,
        }
    }
}

/// Draw `text` with its top-left corner at `(x, y)`, skipping characters that are off the left or
/// right of the screen.
fn draw_clipped(canvas: &mut Canvas, layout: &Layout, text: &str, x: i32, y: i32) {
    let char_width = layout.char_width as i32;
    let first = ((-x).max(0) / char_width) as usize;
    let last = ((hardware::DISPLAY_WIDTH as i32 - x + char_width - 1) / char_width)
        .clamp(0, text.len() as i32) as usize;
    if first >= last {
        return;
    }
    // text from a TextInput is ASCII, so slicing by byte is slicing by character
    Text::with_text_style(
        &text[first..last],
        Point::new(x + first as i32 * char_width, y),
        layout.style,
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
            self.draw_scrolling(canvas, text);
        } else {
            self.draw_still(canvas, text);
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
            Button::Center => {
                self.editing = true;
                self.input.enter();
                Response::Redraw
            }
            Button::Right => {
                self.font = (self.font + 1) % FONTS.len();
                self.scroll = 0;
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
        self.scroll = (self.scroll + step) % self.scroll_period();
        Response::Redraw
    }
}
