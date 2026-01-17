//! Responsive layout system for TUI
//!
//! Provides dynamic terminal scaling with three size categories:
//! - Small: Compact layouts for narrow terminals
//! - Medium: Balanced layouts for standard terminals
//! - Large: Expanded layouts for wide terminals

use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Screen size categories for responsive layouts
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScreenSize {
    /// Small: < 100 cols or < 30 rows
    Small,
    /// Medium: 100-150 cols and 30-50 rows
    Medium,
    /// Large: > 150 cols or > 50 rows
    Large,
}

impl ScreenSize {
    /// Determine screen size from terminal dimensions
    pub fn from_dimensions(width: u16, height: u16) -> Self {
        if width < 100 || height < 30 {
            ScreenSize::Small
        } else if width > 150 || height > 50 {
            ScreenSize::Large
        } else {
            ScreenSize::Medium
        }
    }
}

/// Responsive layout manager
#[derive(Debug, Clone)]
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
        let title_height = match self.size {
            ScreenSize::Small => 1, // Minimal title bar
            ScreenSize::Medium => 3,
            ScreenSize::Large => 3,
        };

        let status_height = match self.size {
            ScreenSize::Small => 1, // Minimal status bar
            ScreenSize::Medium => 3,
            ScreenSize::Large => 3,
        };

        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(title_height),
                Constraint::Min(0),
                Constraint::Length(status_height),
            ])
            .split(area)
            .to_vec()
    }

    /// Get the dashboard layout constraints
    pub fn dashboard_layout(&self, area: Rect) -> Vec<Rect> {
        match self.size {
            ScreenSize::Small => {
                // Stack everything vertically for small screens
                Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(5),  // Network status (compact + sync)
                        Constraint::Length(5),  // Our validators (compact)
                        Constraint::Min(0),     // Recent blocks
                    ])
                    .split(area)
                    .to_vec()
            }
            ScreenSize::Medium => {
                // Standard layout
                Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(6),  // Network status + sync progress
                        Constraint::Length(10), // Our validators + all 3 public keys
                        Constraint::Min(0),     // Recent blocks
                    ])
                    .split(area)
                    .to_vec()
            }
            ScreenSize::Large => {
                // Expanded layout with more detail
                Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(7),   // Network status + sync progress
                        Constraint::Length(12),  // Our validators + all 3 public keys
                        Constraint::Min(0),      // Recent blocks
                    ])
                    .split(area)
                    .to_vec()
            }
        }
    }

    /// Get the enhanced dashboard layout with side-by-side panels (for large screens)
    pub fn dashboard_wide_layout(&self, area: Rect) -> Option<(Vec<Rect>, Vec<Rect>)> {
        if self.size != ScreenSize::Large || self.width < 160 {
            return None;
        }

        // Top row: Network status (left) and Our validators (right)
        let top_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(8),  // Top row
                Constraint::Min(0),     // Recent blocks
            ])
            .split(area);

        let top_horizontal = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50),
                Constraint::Percentage(50),
            ])
            .split(top_chunks[0]);

        Some((top_horizontal.to_vec(), vec![top_chunks[1]]))
    }

    /// Get optimal block count to display based on screen size
    pub fn blocks_to_display(&self) -> usize {
        match self.size {
            ScreenSize::Small => 10,
            ScreenSize::Medium => 20,
            ScreenSize::Large => 30,
        }
    }

    /// Get optimal validator count to display
    pub fn validators_to_display(&self) -> usize {
        match self.size {
            ScreenSize::Small => 15,
            ScreenSize::Medium => 25,
            ScreenSize::Large => 50,
        }
    }

    /// Determine if we should show the full key or truncated version
    pub fn key_display_length(&self) -> KeyDisplayMode {
        match self.size {
            ScreenSize::Small => KeyDisplayMode::VeryShort,  // 8 chars
            ScreenSize::Medium => KeyDisplayMode::Short,     // 12...8
            ScreenSize::Large => KeyDisplayMode::Full,       // Full key
        }
    }

    /// Determine if we should show extra columns in tables
    pub fn show_extra_columns(&self) -> bool {
        matches!(self.size, ScreenSize::Large)
    }

    /// Get column widths for block list
    pub fn block_list_columns(&self) -> BlockListColumns {
        match self.size {
            ScreenSize::Small => BlockListColumns {
                show_slot: false,
                show_epoch: true,
                show_extrinsics: false,
                author_width: 14, // Short display
            },
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
            ScreenSize::Small => ValidatorListColumns {
                key_width: 20,
                show_status: false,
                show_registration: false,
            },
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
    /// Very short: first 4...last 4
    VeryShort,
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
            KeyDisplayMode::VeryShort => {
                if key.len() > 12 {
                    format!("{}...{}", &key[..6], &key[key.len()-4..])
                } else {
                    key.to_string()
                }
            }
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
pub struct BlockListColumns {
    pub show_slot: bool,
    pub show_epoch: bool,
    pub show_extrinsics: bool,
    pub author_width: usize,
}

/// Validator list column configuration
#[derive(Debug, Clone)]
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
        // Small screens
        assert_eq!(ScreenSize::from_dimensions(80, 24), ScreenSize::Small);
        assert_eq!(ScreenSize::from_dimensions(99, 40), ScreenSize::Small);
        assert_eq!(ScreenSize::from_dimensions(120, 29), ScreenSize::Small);

        // Medium screens
        assert_eq!(ScreenSize::from_dimensions(100, 30), ScreenSize::Medium);
        assert_eq!(ScreenSize::from_dimensions(120, 40), ScreenSize::Medium);
        assert_eq!(ScreenSize::from_dimensions(150, 50), ScreenSize::Medium);

        // Large screens
        assert_eq!(ScreenSize::from_dimensions(151, 40), ScreenSize::Large);
        assert_eq!(ScreenSize::from_dimensions(120, 51), ScreenSize::Large);
        assert_eq!(ScreenSize::from_dimensions(200, 60), ScreenSize::Large);
    }

    #[test]
    fn test_key_display_mode() {
        let test_key = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";

        let very_short = KeyDisplayMode::VeryShort.format(test_key);
        assert!(very_short.len() < 20);
        assert!(very_short.contains("..."));

        let short = KeyDisplayMode::Short.format(test_key);
        assert!(short.len() < 30);
        assert!(short.contains("..."));

        let full = KeyDisplayMode::Full.format(test_key);
        assert_eq!(full, test_key);
    }
}
