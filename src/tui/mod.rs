//! Text User Interface (TUI) module for real-time monitoring

mod app;
pub mod event;
mod ui;

pub use app::{App, ViewMode};
pub use event::{Event, EventHandler};
pub use ui::render;
