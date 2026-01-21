//! Responsive layout system for TUI
//!
//! Provides dynamic terminal scaling with two size categories:
//! - Medium: Standard layouts for typical terminals (< 120 cols) - truncated keys
//! - Large: Expanded layouts with full keys (>= 120 cols)
//!
//! The 120 col threshold is based on the widest content line (block list):
//! `#12345678  slot 123456789012  epoch 1234  ✓ author: 0x...66_char_key`
//! which requires 118 chars + 2 for borders = 120 cols to fit without truncation.

use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Screen size categories for responsive layouts
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScreenSize {
    /// Medium: < 120 cols - truncated keys (23 chars: 12...8)
    Medium,
    /// Large: >= 120 cols - full keys (66 chars)
    Large,
}

impl ScreenSize {
    /// Determine screen size from terminal dimensions
    /// Threshold at 120 cols where full 66-char keys fit in block list
    pub fn from_dimensions(width: u16, _height: u16) -> Self {
        if width >= 120 {
            ScreenSize::Large
        } else {
            ScreenSize::Medium
        }
    }
}

/// Responsive layout manager
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ResponsiveLayout {
    pub size: ScreenSize,
    pub width: u16,
    pub height: u16,
}

impl ResponsiveLayout {
    /// Create a new responsive layout from a terminal area
    pub fn new(area: Rect) -> Self {
        Self {
            size: ScreenSize::from_dimensions(area.width, area.height),
            width: area.width,
            height: area.height,
        }
    }

    /// Get the main layout (title bar, content, status bar)
    pub fn main_layout(&self, area: Rect) -> Vec<Rect> {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Title bar
                Constraint::Min(0),     // Content
                Constraint::Length(3),  // Status bar
            ])
            .split(area)
            .to_vec()
    }

    /// Get the dashboard layout constraints
    /// Heights are fixed based on content: Network Status (8 lines + 2 border = 10),
    /// Our Validator (7 lines for 1 validator with 3 keys + 2 border = 9)
    pub fn dashboard_layout(&self, area: Rect, network_status_rows: u16) -> Vec<Rect> {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(network_status_rows + 2), // Content rows + 2 for border
                Constraint::Length(9),  // Our validator: 4 header + 3 keys + 2 border
                Constraint::Min(0),     // Recent blocks: fills remaining space
            ])
            .split(area)
            .to_vec()
    }

    /// Get optimal block count to display based on screen size
    #[allow(dead_code)]
    pub fn blocks_to_display(&self) -> usize {
        match self.size {
            ScreenSize::Medium => 20,
            ScreenSize::Large => 30,
        }
    }

    /// Get optimal validator count to display
    #[allow(dead_code)]
    pub fn validators_to_display(&self) -> usize {
        match self.size {
            ScreenSize::Medium => 25,
            ScreenSize::Large => 50,
        }
    }

    /// Determine if we should show the full key or truncated version
    pub fn key_display_length(&self) -> KeyDisplayMode {
        match self.size {
            ScreenSize::Medium => KeyDisplayMode::Short,  // 12...8
            ScreenSize::Large => KeyDisplayMode::Full,    // Full key
        }
    }

    /// Determine if we should show extra columns in tables
    #[allow(dead_code)]
    pub fn show_extra_columns(&self) -> bool {
        matches!(self.size, ScreenSize::Large)
    }

    /// Get column widths for block list
    pub fn block_list_columns(&self) -> BlockListColumns {
        match self.size {
            ScreenSize::Medium => BlockListColumns {
                show_slot: true,
                show_epoch: true,
                show_extrinsics: true,
                author_width: 20, // Medium display
            },
            ScreenSize::Large => BlockListColumns {
                show_slot: true,
                show_epoch: true,
                show_extrinsics: true,
                author_width: 66, // Full key
            },
        }
    }

    /// Get column widths for validator list
    pub fn validator_list_columns(&self) -> ValidatorListColumns {
        match self.size {
            ScreenSize::Medium => ValidatorListColumns {
                key_width: 66,
                show_status: true,
                show_registration: false,
            },
            ScreenSize::Large => ValidatorListColumns {
                key_width: 66,
                show_status: true,
                show_registration: true,
            },
        }
    }
}

/// Key display mode based on screen size
#[derive(Debug, Clone, Copy)]
pub enum KeyDisplayMode {
    /// Short: first 12...last 8
    Short,
    /// Full: entire key
    Full,
}

impl KeyDisplayMode {
    /// Format a key according to this display mode
    pub fn format(&self, key: &str) -> String {
        let key = key.trim();
        match self {
            KeyDisplayMode::Short => {
                if key.len() > 22 {
                    format!("{}...{}", &key[..12], &key[key.len()-8..])
                } else {
                    key.to_string()
                }
            }
            KeyDisplayMode::Full => key.to_string(),
        }
    }
}

/// Block list column configuration
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct BlockListColumns {
    pub show_slot: bool,
    pub show_epoch: bool,
    pub show_extrinsics: bool,
    pub author_width: usize,
}

/// Validator list column configuration
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ValidatorListColumns {
    pub key_width: usize,
    pub show_status: bool,
    pub show_registration: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_screen_size_detection() {
        // Medium screens (< 120 cols) - truncated keys
        assert_eq!(ScreenSize::from_dimensions(80, 24), ScreenSize::Medium);
        assert_eq!(ScreenSize::from_dimensions(100, 40), ScreenSize::Medium);
        assert_eq!(ScreenSize::from_dimensions(119, 50), ScreenSize::Medium);

        // Large screens (>= 120 cols) - full keys fit
        assert_eq!(ScreenSize::from_dimensions(120, 40), ScreenSize::Large);
        assert_eq!(ScreenSize::from_dimensions(150, 40), ScreenSize::Large);
        assert_eq!(ScreenSize::from_dimensions(200, 60), ScreenSize::Large);
    }

    #[test]
    fn test_key_display_mode() {
        let test_key = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";

        let short = KeyDisplayMode::Short.format(test_key);
        assert!(short.len() < 30);
        assert!(short.contains("..."));

        let full = KeyDisplayMode::Full.format(test_key);
        assert_eq!(full, test_key);
    }
}
