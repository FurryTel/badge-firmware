use embedded_graphics::{
    Drawable,
    geometry::{Point, Size},
    pixelcolor::BinaryColor,
    primitives::{Primitive, PrimitiveStyle, Rectangle},
    text::Text,
};

use super::{Button, Canvas, Response, Screen};

const ROW_HEIGHT: u32 = super::FONT.character_size.height;
const ROWS: usize = (crate::hardware::DISPLAY_HEIGHT / ROW_HEIGHT) as usize;

pub struct Item<'a> {
    pub label: &'static str,
    pub screen: &'a mut dyn Screen,
}

impl<'a> Item<'a> {
    pub fn new(label: &'static str, screen: &'a mut dyn Screen) -> Self {
        Self { label, screen }
    }
}

/// A scrolling list of items, each of which opens another screen.
///
/// Up/Down move the selection, Center/Right open the selected item, and Left exits the menu.  When
/// the opened screen exits, the menu comes back.  A `Menu` is itself a [`Screen`], so submenus are
/// just items whose screen is another `Menu`.
pub struct Menu<'a, const N: usize> {
    items: [Item<'a>; N],
    selected: usize,
    /// whether the selected item's screen is currently being shown
    open: bool,
}

impl<'a, const N: usize> Menu<'a, N> {
    pub fn new(items: [Item<'a>; N]) -> Self {
        const { assert!(N > 0, "a menu needs at least one item") };
        Self {
            items,
            selected: 0,
            open: false,
        }
    }
}

impl<const N: usize> Screen for Menu<'_, N> {
    fn draw(&self, canvas: &mut Canvas) {
        if self.open {
            return self.items[self.selected].screen.draw(canvas);
        }

        // keep the selection in the middle row, except near the ends of the list where that would
        // leave blank rows
        let top = self
            .selected
            .saturating_sub(ROWS / 2)
            .min(N.saturating_sub(ROWS));
        for (row, (i, item)) in self
            .items
            .iter()
            .enumerate()
            .skip(top)
            .take(ROWS)
            .enumerate()
        {
            let y = (row as u32 * ROW_HEIGHT) as i32;
            let style = if i == self.selected {
                Rectangle::new(
                    Point::new(0, y),
                    Size::new(crate::hardware::DISPLAY_WIDTH, ROW_HEIGHT),
                )
                .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                .draw(canvas)
                .unwrap();
                super::TEXT_INVERTED
            } else {
                super::TEXT
            };
            Text::with_text_style(item.label, Point::new(2, y), style, super::TOP_LEFT)
                .draw(canvas)
                .unwrap();
        }
    }

    fn handle(&mut self, button: Button) -> Response {
        if self.open {
            return match self.items[self.selected].screen.handle(button) {
                Response::Exit => {
                    self.open = false;
                    Response::Redraw
                }
                r => r,
            };
        }

        match button {
            Button::Up => self.selected = (self.selected + N - 1) % N,
            Button::Down => self.selected = (self.selected + 1) % N,
            Button::Center | Button::Right => {
                self.open = true;
                self.items[self.selected].screen.enter();
            }
            Button::Left => return Response::Exit,
        }
        Response::Redraw
    }

    fn tick_interval(&self) -> Option<embassy_time::Duration> {
        self.open
            .then(|| self.items[self.selected].screen.tick_interval())
            .flatten()
    }

    fn tick(&mut self) -> Response {
        match self.open {
            true => self.items[self.selected].screen.tick(),
            false => Response::Ignored,
        }
    }
}
