//! Color themes for TUI

use ratatui::style::Color;

/// Theme for the TUI
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Theme {
    Midnight,
    Daytime,
}

impl Theme {
    /// Primary accent color
    pub fn primary(&self) -> Color {
        match self {
            Theme::Midnight => Color::Rgb(138, 102, 255), // Purple/Violet
            Theme::Daytime => Color::Rgb(75, 50, 180),    // Darker purple for light mode
        }
    }

    /// Secondary accent color
    pub fn secondary(&self) -> Color {
        match self {
            Theme::Midnight => Color::Rgb(102, 153, 255), // Light blue
            Theme::Daytime => Color::Rgb(50, 100, 200),   // Darker blue for light mode
        }
    }

    /// Success/positive color
    pub fn success(&self) -> Color {
        match self {
            Theme::Midnight => Color::Rgb(120, 255, 180), // Mint green
            Theme::Daytime => Color::Rgb(0, 150, 80),     // Darker green for light mode
        }
    }

    /// Warning color
    pub fn warning(&self) -> Color {
        match self {
            Theme::Midnight => Color::Rgb(255, 200, 100), // Warm yellow/orange
            Theme::Daytime => Color::Rgb(200, 120, 0),    // Darker orange for light mode
        }
    }

    /// Error/alert color
    pub fn error(&self) -> Color {
        match self {
            Theme::Midnight => Color::Rgb(255, 100, 120), // Soft red
            Theme::Daytime => Color::Rgb(200, 0, 40),     // Darker red for light mode
        }
    }

    /// Muted/secondary text color
    pub fn muted(&self) -> Color {
        match self {
            Theme::Midnight => Color::Rgb(150, 150, 170), // Light gray-purple
            Theme::Daytime => Color::Rgb(100, 100, 120),  // Darker gray for light mode
        }
    }

    /// Highlight color for selected items
    pub fn highlight(&self) -> Color {
        match self {
            Theme::Midnight => Color::Rgb(60, 45, 90),   // Dark purple
            Theme::Daytime => Color::Rgb(220, 220, 240), // Light purple-gray
        }
    }

    /// Border color
    pub fn border(&self) -> Color {
        match self {
            Theme::Midnight => Color::Rgb(100, 80, 140), // Purple-gray
            Theme::Daytime => Color::Rgb(180, 180, 200), // Light gray
        }
    }

    /// Title color
    pub fn title(&self) -> Color {
        match self {
            Theme::Midnight => Color::Rgb(180, 150, 255), // Bright purple
            Theme::Daytime => Color::Rgb(60, 40, 150),    // Dark purple
        }
    }

    /// Our validator indicator color
    pub fn ours(&self) -> Color {
        match self {
            Theme::Midnight => Color::Rgb(255, 215, 100), // Gold
            Theme::Daytime => Color::Rgb(200, 140, 0),    // Dark gold
        }
    }

    /// Epoch/slot color
    pub fn epoch(&self) -> Color {
        match self {
            Theme::Midnight => Color::Rgb(150, 200, 255), // Light cyan-blue
            Theme::Daytime => Color::Rgb(40, 80, 150),    // Dark blue
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
            Theme::Daytime => Color::Rgb(40, 40, 50),     // Dark gray
        }
    }

    /// Toggle to the other theme
    pub fn toggle(&self) -> Theme {
        match self {
            Theme::Midnight => Theme::Daytime,
            Theme::Daytime => Theme::Midnight,
        }
    }

    /// Get theme name as string
    pub fn name(&self) -> &'static str {
        match self {
            Theme::Midnight => "Midnight Mode",
            Theme::Daytime => "Daytime Mode",
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Theme::Midnight
    }
}
