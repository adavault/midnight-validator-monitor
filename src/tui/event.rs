//! Event handling for TUI

use anyhow::Result;
use crossterm::event::{self, Event as CrosstermEvent, KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;

/// Terminal events
#[derive(Debug, Clone, Copy)]
pub enum Event {
    /// Key press event
    Key(KeyEvent),
    /// Tick event for periodic updates
    Tick,
    /// Resize event
    Resize,
}

/// Event handler for terminal events
pub struct EventHandler {
    /// Tick rate for periodic updates
    tick_rate: Duration,
}

impl EventHandler {
    /// Create a new event handler
    pub fn new(tick_rate: Duration) -> Self {
        Self { tick_rate }
    }

    /// Read next event (blocking with timeout)
    pub fn next(&self) -> Result<Event> {
        if event::poll(self.tick_rate)? {
            match event::read()? {
                CrosstermEvent::Key(key) => Ok(Event::Key(key)),
                CrosstermEvent::Resize(_, _) => Ok(Event::Resize),
                _ => Ok(Event::Tick),
            }
        } else {
            Ok(Event::Tick)
        }
    }
}

/// Parse keyboard event and return whether to continue running
pub fn handle_key_event(key: KeyEvent, app: &mut crate::tui::App) -> bool {
    match key.code {
        // Quit
        KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
            app.quit();
            false
        }
        // Ctrl+C
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.quit();
            false
        }
        // View switching with numbers
        KeyCode::Char('1') => {
            app.set_view(crate::tui::ViewMode::Dashboard);
            true
        }
        KeyCode::Char('2') => {
            app.set_view(crate::tui::ViewMode::Blocks);
            true
        }
        KeyCode::Char('3') => {
            app.set_view(crate::tui::ViewMode::Validators);
            true
        }
        KeyCode::Char('4') => {
            app.set_view(crate::tui::ViewMode::Performance);
            true
        }
        KeyCode::Char('5') => {
            app.set_view(crate::tui::ViewMode::Peers);
            true
        }
        KeyCode::Char('?') | KeyCode::Char('h') | KeyCode::F(1) => {
            app.set_view(crate::tui::ViewMode::Help);
            true
        }
        // Navigation
        KeyCode::Tab => {
            app.next_view();
            true
        }
        KeyCode::BackTab => {
            app.previous_view();
            true
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.scroll_down();
            true
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.scroll_up();
            true
        }
        // Toggle filters
        KeyCode::Char('f') | KeyCode::Char('F') => {
            app.toggle_ours_filter();
            true
        }
        // Toggle theme
        KeyCode::Char('t') | KeyCode::Char('T') => {
            app.toggle_theme();
            true
        }
        _ => true,
    }
}
