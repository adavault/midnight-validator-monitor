# Release Notes - v0.4.0-beta

## Overview

v0.4.0-beta is a major release focusing on TUI excellence and enhanced monitoring capabilities. This release includes the block attribution fixes from v0.4.0-alpha plus significant TUI improvements.

## Major Features

### 1. Dynamic Terminal Scaling

The TUI now automatically adapts to different terminal sizes with three responsive layouts:

- **Small** (< 100 cols or < 30 rows): Compact layouts with abbreviated text
- **Medium** (100-150 cols, 30-50 rows): Standard layouts with full information
- **Large** (> 150 cols or > 50 rows): Expanded layouts with additional details

**Features**:
- Automatic screen size detection
- Responsive key display (abbreviated for small screens, full for large)
- Dynamic column visibility based on available space
- Compact status bars and titles for constrained terminals

### 2. Enhanced Dashboard View

The dashboard now includes comprehensive monitoring information:

**Network Status Panel**:
- Health indicator (● for healthy, ○ for syncing)
- Peer count display
- Epoch progress bar with visual indicator
- Block and finalization status

**Our Validators Panel**:
- Current epoch block count vs expected
- Performance indicator (✓ on-track, ○ slightly behind, ! behind)
- All-time block statistics
- Share percentage

**Example Progress Bar**:
```
Epoch Progress: ━━━━━━━━━━━━━━━━░░░░ 67.5%
This Epoch: 11 blocks  (expected: ~15.2) ✓
```

### 3. Block Prediction Algorithm

New prediction module for accurate block production forecasting:

```rust
// Example: Calculate expected blocks for validator with 10 committee seats
let calc = PredictionCalculator::new(7200, 1200); // epoch_slots, committee_size
let prediction = calc.calculate(10, 0.5, 28); // seats, epoch_progress, actual
// prediction.expected_blocks = 30.0
// prediction.performance_ratio = Some(0.93)
```

**Features**:
- Uses committee size for accurate predictions
- Confidence intervals based on binomial distribution
- Performance ratio calculation (actual / expected)
- Status indicators: excellent, good, warning, poor

### 4. Block History Sparkline Widget

New visual representation of block production history:

```
Block History (Last 24 hours):
▁▂▃▄▅▆▇█▇▆▅▄▃▂▁▂▃▄▅▆▇█▇▆
```

**Features**:
- Unicode block characters for visual representation
- Automatic scaling based on maximum value
- Configurable width
- Styled line output with custom colors

## Block Attribution Fix (from v0.4.0-alpha)

**Critical fix**: Blocks now correctly use the 1200-seat AURA committee instead of 185 registered candidates for author attribution.

**Key changes**:
- `fetch_with_committee()` queries `AuraApi_authorities` via `state_call`
- Historical committee queries pass block hash for accurate attribution
- Fallback mechanism for pruned/non-archive nodes

See RELEASE_NOTES_v0.4.0-alpha.md for full details.

## New Files

### TUI Enhancements
- `src/tui/layout.rs` - Responsive layout system with ScreenSize enum
- `src/tui/widgets/mod.rs` - Widget module exports
- `src/tui/widgets/sparkline.rs` - Block history sparkline widget

### Midnight Module
- `src/midnight/prediction.rs` - Block prediction algorithm

## Modified Files

- `src/tui/mod.rs` - Export new modules and types
- `src/tui/app.rs` - Added EpochProgress struct and enhanced state tracking
- `src/tui/ui.rs` - Responsive rendering for all views

## API Additions

### EpochProgress Struct
```rust
pub struct EpochProgress {
    pub current_slot_in_epoch: u64,
    pub epoch_length_slots: u64,
    pub progress_percent: f64,
    pub our_blocks_this_epoch: u64,
    pub expected_blocks: f64,
    pub committee_size: u64,
    pub our_committee_seats: u64,
}
```

### PredictionCalculator
```rust
impl PredictionCalculator {
    pub fn new(epoch_length_slots: u64, committee_size: u64) -> Self;
    pub fn calculate(&self, seats: u64, epoch_progress: f64, actual_blocks: u64) -> BlockPrediction;
    pub fn expected_for_full_epoch(&self, seats: u64) -> f64;
    pub fn performance_status(prediction: &BlockPrediction) -> &'static str;
}
```

### BlockHistory
```rust
impl BlockHistory {
    pub fn new(time_range: &str) -> Self;
    pub fn add_entry(&mut self, label: &str, block_count: u64, expected: Option<f64>);
    pub fn render_sparkline(&self, width: usize) -> String;
    pub fn render_styled_line(&self, width: usize, bar_color: Color, empty_color: Color) -> Line<'static>;
}
```

## Testing

All 33 tests pass:
- 6 prediction algorithm tests
- 4 sparkline widget tests
- 2 responsive layout tests
- 21 existing tests

## Upgrade Notes

### From v0.3.0-alpha
1. Database will be automatically updated with new schema
2. Recommend deleting existing database for accurate block attribution
3. TUI will automatically adapt to your terminal size

### Configuration
No configuration changes required. New features are automatically enabled.

## Known Limitations

1. **Block history sparkline**: Currently shows static data; will be connected to database in future release
2. **Health check history**: Tracking infrastructure not yet implemented
3. **Committee seat tracking**: Real-time committee seat count requires live RPC calls

## Performance

- Responsive layout calculations add < 1ms per render
- Prediction calculations are O(1)
- Sparkline rendering is O(n) where n is history entries

## What's Next (v0.5 Roadmap)

1. Health check history with persistent storage
2. Real-time committee seat tracking via RPC
3. Block history from database with configurable time ranges
4. Prometheus metrics export
5. Alert webhooks for missed blocks
