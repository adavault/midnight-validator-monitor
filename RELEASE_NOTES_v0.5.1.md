# Release Notes - v0.5.1

## Overview

v0.5.1 is a polish release focusing on TUI improvements, layout simplification, and bug fixes.

## Changes

### Layout Simplification

- **Reduced to 2 responsive layouts**: Removed the very narrow and very wide layouts, keeping only Medium (< 120 cols) and Large (>= 120 cols)
- **Smart threshold**: The 120-column threshold is based on actual content width - full 66-character keys fit without truncation at >= 120 columns
- **Consistent panel heights**: Network Status and Our Validator panels now have fixed heights that don't change between layouts

### TUI Improvements

- **Renamed "Our Validators" to "Our Validator"**: Singular form for cleaner display
- **Clearer epoch display**: Mainchain and Sidechain epochs now on separate lines with the progress bar clearly associated with Mainchain
- **Removed redundant "Count" field**: The validator count was redundant when showing validator details below
- **Theme names**: "Midnight Mode" renamed to "Midnight Theme", "Daytime" renamed to "Midday Theme"

### Bug Fixes

- **Fixed "This Epoch" block count**: Was incorrectly counting from only the last 20 blocks instead of querying the database for all blocks in the current epoch. Now correctly shows blocks produced by your validator in the current mainchain epoch.

### Display Changes

**Network Status panel (before):**
```
Mainchain: epoch 539    Sidechain: epoch 539  slot 46627312
Epoch Progress: ━━━━━━━━━━━━━━━━━━━━ 65.2%   MVM Db: 12345 blocks
```

**Network Status panel (after):**
```
Mainchain:  epoch 539  ━━━━━━━━━━━━━━━━━━━━ 65.2%
Sidechain:  epoch 539  slot 46627312      MVM Db: 12345 blocks
```

**Our Validator panel (before):**
```
Committee: ✓ Elected (1 seat in 1200 member committee)
Count: 1      All-Time Blocks: 24      Share: 0.050%
This Epoch: 1 blocks  (expected: ~1.2)  ✓
```

**Our Validator panel (after):**
```
Committee: ✓ Elected (1 seat in 1200 member committee)
All-Time Blocks: 24      Share: 0.050%
This Epoch: 1 blocks  (expected: ~1.2)  ✓
```

## Technical Details

### Layout Thresholds

| Screen Width | Layout | Key Display |
|--------------|--------|-------------|
| < 120 cols   | Medium | Truncated (23 chars: `0x1234567890ab...12345678`) |
| >= 120 cols  | Large  | Full (66 chars) |

### Panel Heights (Fixed)

- Network Status: 6 lines (4 content + 2 border)
- Our Validator: 8 lines (6 content + 2 border)
- Recent Blocks: Fills remaining space

## Files Changed

- `Cargo.toml` - Version bump to 0.5.1
- `src/tui/ui.rs` - Layout fixes, epoch display improvements, label changes
- `src/tui/layout.rs` - Simplified to 2 layouts, threshold changed to 120 cols
- `src/tui/app.rs` - Fixed epoch block counting to use database query

## Upgrade Instructions

1. **Build the new version:**
   ```bash
   cargo build --release
   ```

2. **Install the binary:**
   ```bash
   sudo cp target/release/mvm /opt/midnight/mvm/bin/mvm
   ```

3. **Verify the installation:**
   ```bash
   mvm --version
   mvm view
   ```

## Testing

All 33 unit tests pass. Manual testing verified:
- Layout transitions at 120 column threshold
- Correct "This Epoch" block counting
- Theme switching between Midnight and Midday
- Panel heights consistent across layouts
