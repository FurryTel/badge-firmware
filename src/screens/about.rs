use embedded_graphics::{
    Drawable,
    geometry::Point,
    text::{Alignment, Baseline, Text, TextStyleBuilder},
};

use crate::hardware;
use crate::ui::{self, Button, Canvas, Response, Screen};

/// Three lines per page, which is all that fits in [`ui::FONT`].
const PAGES: &[[&str; 3]] = &[
    [
        "Use these     ▲  ",
        "buttons to  ◀ ● ▶",
        "navigate!     ▼  "
    ],
    [
        "FurryTel badge",
        concat!("v", env!("CARGO_PKG_VERSION")),
        "furrytel.net/badge",
    ],
];

/// Pages of text: Right/Down for the next page, Up for the previous one, Left/Center exits.
pub struct About {
    page: usize,
}

impl About {
    pub fn new() -> Self {
        Self { page: 0 }
    }
}

impl Screen for About {
    fn enter(&mut self) {
        self.page = 0;
    }

    fn draw(&self, canvas: &mut Canvas) {
        for (i, line) in PAGES[self.page].into_iter().enumerate() {
            let y = i as i32 * ui::FONT.character_size.height as i32;
            Text::with_text_style(line, Point::new(0, y), ui::TEXT, ui::TOP_LEFT)
                .draw(canvas)
                .unwrap();
        }

        // page number in the bottom-right corner
        let mut number = heapless::String::<8>::new();
        core::fmt::Write::write_fmt(
            &mut number,
            format_args!("{}/{}", self.page + 1, PAGES.len()),
        )
        .unwrap();
        Text::with_text_style(
            &number,
            Point::new(
                hardware::DISPLAY_WIDTH as i32 - 1,
                hardware::DISPLAY_HEIGHT as i32 - 1,
            ),
            ui::SMALL_TEXT,
            TextStyleBuilder::new()
                .alignment(Alignment::Right)
                .baseline(Baseline::Bottom)
                .build(),
        )
        .draw(canvas)
        .unwrap();
    }

    fn handle(&mut self, button: Button) -> Response {
        self.page += 1;
        if self.page >= PAGES.len() {
            return Response::Exit;
        }
        Response::Redraw
    }
}
