//! Event handling for TUI

use crate::db::Database;
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
/// The `db` parameter is optional - if not provided, validator detail won't work
pub fn handle_key_event(key: KeyEvent, app: &mut crate::tui::App, db: Option<&Database>) -> bool {
    use crate::tui::{PopupContent, ViewMode};

    // Handle popup-specific keys first
    if app.has_popup() {
        // Check if it's a scrollable popup (ValidatorDetail)
        let is_scrollable = matches!(app.popup, Some(PopupContent::ValidatorDetail { .. }));

        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                app.close_popup();
                return true;
            }
            // Scrolling for validator detail popup
            KeyCode::Down | KeyCode::Char('j') if is_scrollable => {
                app.popup_scroll_down();
                return true;
            }
            KeyCode::Up | KeyCode::Char('k') if is_scrollable => {
                app.popup_scroll_up();
                return true;
            }
            KeyCode::Char('J') | KeyCode::PageDown if is_scrollable => {
                app.popup_page_down();
                return true;
            }
            KeyCode::Char('K') | KeyCode::PageUp if is_scrollable => {
                app.popup_page_up();
                return true;
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.quit();
                return false;
            }
            _ => return true, // Ignore other keys when popup is open
        }
    }

    match key.code {
        // Quit - but only from main views (popup/drill-down handled above)
        KeyCode::Char('q') | KeyCode::Char('Q') => {
            app.quit();
            false
        }
        // Escape - close popup, pop drill-down, or quit
        KeyCode::Esc => {
            if app.can_pop() {
                app.pop_view();
            } else {
                app.quit();
            }
            !app.should_quit
        }
        // Ctrl+C
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.quit();
            false
        }
        // Enter - open popup based on current view
        KeyCode::Enter => {
            match app.view_mode {
                ViewMode::Blocks => {
                    app.open_block_popup();
                }
                ViewMode::Validators => {
                    app.open_validator_identity_popup(db);
                }
                ViewMode::Performance => {
                    if let Some(db) = db {
                        app.open_validator_popup(db);
                    }
                }
                ViewMode::Peers => {
                    app.open_peer_popup();
                }
                _ => {}
            }
            true
        }
        // View switching with numbers
        KeyCode::Char('1') => {
            app.set_view(ViewMode::Dashboard);
            true
        }
        KeyCode::Char('2') => {
            app.set_view(ViewMode::Blocks);
            true
        }
        KeyCode::Char('3') => {
            app.set_view(ViewMode::Validators);
            true
        }
        KeyCode::Char('4') => {
            app.set_view(ViewMode::Performance);
            true
        }
        KeyCode::Char('5') => {
            app.set_view(ViewMode::Peers);
            true
        }
        KeyCode::Char('?') | KeyCode::Char('h') | KeyCode::F(1) => {
            app.set_view(ViewMode::Help);
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
        // Backspace - pop view stack if in drill-down
        KeyCode::Backspace => {
            if app.can_pop() {
                app.pop_view();
            }
            true
        }
        // Page scroll - J/K (uppercase) or PageUp/PageDown
        KeyCode::Char('J') | KeyCode::PageDown => {
            app.scroll_page_down();
            true
        }
        KeyCode::Char('K') | KeyCode::PageUp => {
            app.scroll_page_up();
            true
        }
        // Single line scroll - j/k or arrow keys
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
