use core::fmt::Write as _;

use embedded_graphics::{
    Drawable,
    geometry::Point,
    text::{Alignment, Baseline, Text, TextStyleBuilder},
};

use crate::ui::{self, Button, Canvas, Response, Screen};
use crate::{backlight, discrete, hardware};

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

/// Edit the backlight color and discrete LED brightnesses.
///
/// ```text
/// Backlight: rgb! 00 00 00
///  Discrete: rand 00 00 00
/// ```
///
/// Left/Right pick a field, Up/Down change it, Center exits.  The first field on each row is its
/// mode: the backlight's rainbow ("rgb!") and the discrete LEDs' random blinking ("rand"), or
/// "pick" to use the values on the rest of the row.  Changing one of those values switches its row
/// to "pick".
pub struct LedColors {
    rainbow: bool,
    backlight_values: [Value<0x11>; 3],
    blink: bool,
    led_values: [Value<1>; 3],
    highlighted: usize,
}

impl LedColors {
    /// Each row is a mode then three values: backlight (red, green, blue), then discrete (top,
    /// middle, bottom).
    const FIELDS_PER_ROW: usize = 4;
    const FIELDS: usize = 2 * Self::FIELDS_PER_ROW;

    const LABEL_COLUMNS: i32 = "Backlight:".len() as i32;
    const MODE_X: i32 = Self::LABEL_COLUMNS * ui::FONT.character_size.width as i32 + 1;
    /// x of each value column, packed in to fit after the mode field on a 128px screen
    const VALUE_X: [i32; 3] = [87, 101, 115];
    const ROW_Y: [i32; 2] = [0, 15];
    const HELP: &str = "</> sel, ^/v edit, o quit";

    pub fn new() -> Self {
        let mut this = Self {
            rainbow: true,
            backlight_values: [Value { value: 0x88 }; 3],
            blink: true,
            led_values: Default::default(),
            highlighted: 0,
        };
        this.apply();
        this
    }

    fn step(&mut self, up: bool) {
        let row = self.highlighted / Self::FIELDS_PER_ROW;
        let column = self.highlighted % Self::FIELDS_PER_ROW;
        let (mode, value): (&mut bool, &mut dyn Steppable) = match (row, column) {
            (0, 0) => return self.rainbow = !self.rainbow,
            (_, 0) => return self.blink = !self.blink,
            (0, c) => (&mut self.rainbow, &mut self.backlight_values[c - 1]),
            (_, c) => (&mut self.blink, &mut self.led_values[c - 1]),
        };
        *mode = false;
        if up { value.inc() } else { value.dec() }
    }

    fn apply(&mut self) {
        backlight::set(if self.rainbow {
            backlight::Mode::Rainbow
        } else {
            backlight::Mode::Solid(self.backlight_values.map(|v| v.value))
        });
        discrete::set(if self.blink {
            discrete::Mode::Random
        } else {
            discrete::Mode::Solid(self.led_values.map(|v| v.value))
        });
    }
}

impl Screen for LedColors {
    fn draw(&self, canvas: &mut Canvas) {
        let style = |field| {
            if field == self.highlighted {
                ui::TEXT_HIGHLIGHTED
            } else {
                ui::TEXT
            }
        };

        let rows = [
            (
                "Backlight:",
                if self.rainbow { "rgb!" } else { "pick" },
                self.backlight_values.map(|v| v.value),
            ),
            (
                "Discrete:",
                if self.blink { "rand" } else { "pick" },
                self.led_values.map(|v| v.value),
            ),
        ];
        for (row, (label, mode, values)) in rows.into_iter().enumerate() {
            let y = Self::ROW_Y[row];
            let first_field = row * Self::FIELDS_PER_ROW;

            // right-aligned so the colons line up
            let x =
                (Self::LABEL_COLUMNS - label.len() as i32) * ui::FONT.character_size.width as i32;
            Text::with_text_style(label, Point::new(x, y), ui::TEXT, ui::TOP_LEFT)
                .draw(canvas)
                .unwrap();

            Text::with_text_style(
                mode,
                Point::new(Self::MODE_X, y),
                style(first_field),
                ui::TOP_LEFT,
            )
            .draw(canvas)
            .unwrap();

            for (i, value) in values.into_iter().enumerate() {
                let mut text = heapless::String::<2>::new();
                core::write!(&mut text, "{:02x}", value).unwrap();
                let position = Point::new(Self::VALUE_X[i], y);
                Text::with_text_style(&text, position, style(first_field + 1 + i), ui::TOP_LEFT)
                    .draw(canvas)
                    .unwrap();
            }
        }

        // in the small font, to squeeze in under the two rows
        Text::with_text_style(
            Self::HELP,
            Point::new(
                hardware::DISPLAY_WIDTH as i32 / 2,
                hardware::DISPLAY_HEIGHT as i32 - 1,
            ),
            ui::SMALL_TEXT,
            TextStyleBuilder::new()
                .alignment(Alignment::Center)
                .baseline(Baseline::Bottom)
                .build(),
        )
        .draw(canvas)
        .unwrap();
    }

    fn handle(&mut self, button: Button) -> Response {
        match button {
            Button::Up => self.step(true),
            Button::Down => self.step(false),
            Button::Left => self.highlighted = (self.highlighted + Self::FIELDS - 1) % Self::FIELDS,
            Button::Right => self.highlighted = (self.highlighted + 1) % Self::FIELDS,
            Button::Center => return Response::Exit,
        }
        self.apply();
        Response::Redraw
    }
}
