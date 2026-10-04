//! The badge's actual screens, built on [`crate::ui`].

mod about;
mod bootloader;
mod led_colors;
mod text_display;

pub use about::About;
pub use bootloader::Bootloader;
pub use led_colors::LedColors;
pub use text_display::TextDisplay;
