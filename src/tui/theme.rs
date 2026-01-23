//! Color themes for TUI

use ratatui::style::Color;

/// Theme for the TUI
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Theme {
    #[default]
    Midnight,
    Midday,
}

impl Theme {
    /// Primary accent color
    pub fn primary(&self) -> Color {
        match self {
            Theme::Midnight => Color::Rgb(138, 102, 255), // Purple/Violet
            Theme::Midday => Color::Rgb(0, 150, 200),     // Vibrant teal/cyan
        }
    }

    /// Secondary accent color
    pub fn secondary(&self) -> Color {
        match self {
            Theme::Midnight => Color::Rgb(102, 153, 255), // Light blue
            Theme::Midday => Color::Rgb(0, 80, 180),      // Vivid blue (readable on light bg)
        }
    }

    /// Success/positive color
    pub fn success(&self) -> Color {
        match self {
            Theme::Midnight => Color::Rgb(120, 255, 180), // Mint green
            Theme::Midday => Color::Rgb(0, 180, 100),     // Vibrant emerald
        }
    }

    /// Warning color
    pub fn warning(&self) -> Color {
        match self {
            Theme::Midnight => Color::Rgb(255, 200, 100), // Warm yellow/orange
            Theme::Midday => Color::Rgb(255, 140, 0),     // Bright orange
        }
    }

    /// Error/alert color
    pub fn error(&self) -> Color {
        match self {
            Theme::Midnight => Color::Rgb(255, 100, 120), // Soft red
            Theme::Midday => Color::Rgb(220, 50, 80),     // Vibrant coral red
        }
    }

    /// Muted/secondary text color
    pub fn muted(&self) -> Color {
        match self {
            Theme::Midnight => Color::Rgb(150, 150, 170), // Light gray-purple
            Theme::Midday => Color::Rgb(100, 115, 140),   // Medium slate (brighter)
        }
    }

    /// Highlight color for selected items
    pub fn highlight(&self) -> Color {
        match self {
            Theme::Midnight => Color::Rgb(60, 45, 90),  // Dark purple
            Theme::Midday => Color::Rgb(210, 235, 255), // Light sky blue
        }
    }

    /// Border color
    pub fn border(&self) -> Color {
        match self {
            Theme::Midnight => Color::Rgb(100, 80, 140), // Purple-gray
            Theme::Midday => Color::Rgb(140, 170, 200),  // Light steel blue
        }
    }

    /// Title color
    pub fn title(&self) -> Color {
        match self {
            Theme::Midnight => Color::Rgb(180, 150, 255), // Bright purple
            Theme::Midday => Color::Rgb(0, 140, 200),     // Bright teal
        }
    }

    /// Our validator indicator color
    pub fn ours(&self) -> Color {
        match self {
            Theme::Midnight => Color::Rgb(255, 215, 100), // Gold
            Theme::Midday => Color::Rgb(230, 140, 0),     // Amber orange
        }
    }

    /// Epoch/slot color
    pub fn epoch(&self) -> Color {
        match self {
            Theme::Midnight => Color::Rgb(150, 200, 255), // Light cyan-blue
            Theme::Midday => Color::Rgb(130, 80, 200),    // Bright purple (readable)
        }
    }

    /// Block number color
    pub fn block_number(&self) -> Color {
        self.secondary()
    }

    /// Normal text color
    pub fn text(&self) -> Color {
        match self {
            Theme::Midnight => Color::Rgb(220, 220, 230), // Light gray
            Theme::Midday => Color::Rgb(50, 60, 80),      // Medium navy (brighter)
        }
    }

    /// Toggle to the other theme
    pub fn toggle(&self) -> Theme {
        match self {
            Theme::Midnight => Theme::Midday,
            Theme::Midday => Theme::Midnight,
        }
    }

    /// Get theme name as string
    pub fn name(&self) -> &'static str {
        match self {
            Theme::Midnight => "Midnight Theme",
            Theme::Midday => "Midday Theme",
        }
    }
}
