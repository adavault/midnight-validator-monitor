//! Block history sparkline widget
//!
//! Displays block production history as a visual sparkline chart.

use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};

/// Block production entry for a time period
#[derive(Debug, Clone, Default)]
pub struct BlockHistoryEntry {
    /// Time period label (e.g., "00", "01" for hours)
    pub label: String,
    /// Number of blocks produced in this period
    pub block_count: u64,
    /// Expected blocks for this period (optional)
    pub expected_count: Option<f64>,
}

/// Block history data for sparkline display
#[derive(Debug, Clone, Default)]
pub struct BlockHistory {
    /// Historical entries (oldest first)
    pub entries: Vec<BlockHistoryEntry>,
    /// Maximum blocks in any single period (for scaling)
    pub max_blocks: u64,
    /// Time range description (e.g., "Last 24 hours")
    pub time_range: String,
}

impl BlockHistory {
    /// Create a new empty block history
    pub fn new(time_range: &str) -> Self {
        Self {
            entries: Vec::new(),
            max_blocks: 0,
            time_range: time_range.to_string(),
        }
    }

    /// Add an entry to the history
    pub fn add_entry(&mut self, label: &str, block_count: u64, expected: Option<f64>) {
        if block_count > self.max_blocks {
            self.max_blocks = block_count;
        }
        self.entries.push(BlockHistoryEntry {
            label: label.to_string(),
            block_count,
            expected_count: expected,
        });
    }

    /// Get the total blocks across all entries
    pub fn total_blocks(&self) -> u64 {
        self.entries.iter().map(|e| e.block_count).sum()
    }

    /// Render as a sparkline string with blocks
    ///
    /// Uses Unicode block characters:
    /// - ' ' (empty) for 0
    /// - '▁' for 1-12.5%
    /// - '▂' for 12.5-25%
    /// - '▃' for 25-37.5%
    /// - '▄' for 37.5-50%
    /// - '▅' for 50-62.5%
    /// - '▆' for 62.5-75%
    /// - '▇' for 75-87.5%
    /// - '█' for 87.5-100%
    pub fn render_sparkline(&self, width: usize) -> String {
        const BLOCKS: [char; 9] = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

        if self.entries.is_empty() || self.max_blocks == 0 {
            return "░".repeat(width);
        }

        // If we have more entries than width, sample them
        let entries: Vec<u64> = if self.entries.len() > width {
            let step = self.entries.len() as f64 / width as f64;
            (0..width)
                .map(|i| {
                    let start = (i as f64 * step) as usize;
                    let end = ((i + 1) as f64 * step) as usize;
                    // Average the entries in this range
                    self.entries[start..end.min(self.entries.len())]
                        .iter()
                        .map(|e| e.block_count)
                        .sum::<u64>()
                        / (end - start).max(1) as u64
                })
                .collect()
        } else {
            // Pad with zeros if we have fewer entries
            let mut data: Vec<u64> = self.entries.iter().map(|e| e.block_count).collect();
            while data.len() < width {
                data.insert(0, 0);
            }
            data
        };

        entries
            .iter()
            .map(|&count| {
                if count == 0 {
                    BLOCKS[0]
                } else {
                    let normalized = (count as f64 / self.max_blocks as f64 * 8.0) as usize;
                    BLOCKS[normalized.min(8)]
                }
            })
            .collect()
    }

    /// Render as styled spans for ratatui
    pub fn render_styled_line(
        &self,
        width: usize,
        bar_color: Color,
        empty_color: Color,
    ) -> Line<'static> {
        const BLOCKS: [char; 9] = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

        if self.entries.is_empty() || self.max_blocks == 0 {
            return Line::from(vec![
                Span::styled("░".repeat(width), Style::default().fg(empty_color))
            ]);
        }

        // If we have more entries than width, sample them
        let entries: Vec<u64> = if self.entries.len() > width {
            let step = self.entries.len() as f64 / width as f64;
            (0..width)
                .map(|i| {
                    let start = (i as f64 * step) as usize;
                    let end = ((i + 1) as f64 * step) as usize;
                    self.entries[start..end.min(self.entries.len())]
                        .iter()
                        .map(|e| e.block_count)
                        .sum::<u64>()
                        / (end - start).max(1) as u64
                })
                .collect()
        } else {
            let mut data: Vec<u64> = self.entries.iter().map(|e| e.block_count).collect();
            while data.len() < width {
                data.insert(0, 0);
            }
            data
        };

        let spans: Vec<Span> = entries
            .iter()
            .map(|&count| {
                if count == 0 {
                    Span::styled(BLOCKS[0].to_string(), Style::default().fg(empty_color))
                } else {
                    let normalized = (count as f64 / self.max_blocks as f64 * 8.0) as usize;
                    Span::styled(BLOCKS[normalized.min(8)].to_string(), Style::default().fg(bar_color))
                }
            })
            .collect();

        Line::from(spans)
    }

    /// Create a sample block history from recent blocks
    ///
    /// Groups blocks by hour for the last N hours
    pub fn from_blocks_by_hour(
        blocks: &[(u64, u64)], // (block_number, timestamp)
        hours: usize,
    ) -> Self {
        let now = chrono::Utc::now().timestamp() as u64;
        let hour_seconds = 3600u64;

        let mut history = BlockHistory::new(&format!("Last {} hours", hours));

        for hour in (0..hours).rev() {
            let hour_start = now.saturating_sub((hour + 1) as u64 * hour_seconds);
            let hour_end = now.saturating_sub(hour as u64 * hour_seconds);

            let count = blocks.iter()
                .filter(|(_, ts)| *ts >= hour_start && *ts < hour_end)
                .count() as u64;

            let hour_label = format!("{:02}", (24 - hours + hour) % 24);
            history.add_entry(&hour_label, count, None);
        }

        history
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_history() {
        let history = BlockHistory::new("Test");
        assert_eq!(history.render_sparkline(10), "░░░░░░░░░░");
    }

    #[test]
    fn test_single_entry() {
        let mut history = BlockHistory::new("Test");
        history.add_entry("00", 5, None);

        let sparkline = history.render_sparkline(5);
        assert!(!sparkline.is_empty());
        // Should have 4 empty + 1 full block (5 width, padded left)
        assert_eq!(sparkline.chars().last().unwrap(), '█');
    }

    #[test]
    fn test_varying_heights() {
        let mut history = BlockHistory::new("Test");
        history.add_entry("00", 1, None);
        history.add_entry("01", 4, None);
        history.add_entry("02", 8, None);
        history.add_entry("03", 2, None);

        let sparkline = history.render_sparkline(4);
        assert_eq!(sparkline.chars().count(), 4);
        // Highest (8) should be full block
        assert!(sparkline.contains('█'));
    }

    #[test]
    fn test_total_blocks() {
        let mut history = BlockHistory::new("Test");
        history.add_entry("00", 5, None);
        history.add_entry("01", 3, None);
        history.add_entry("02", 2, None);

        assert_eq!(history.total_blocks(), 10);
    }
}
