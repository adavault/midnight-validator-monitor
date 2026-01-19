//! Text User Interface (TUI) module for real-time monitoring

mod app;
pub mod event;
mod layout;
mod theme;
mod ui;

pub use app::{App, ViewMode, PopupContent};
pub use event::{Event, EventHandler};
pub use layout::ScreenSize;
pub use theme::Theme;
pub use ui::render;
