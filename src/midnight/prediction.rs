//! Block prediction algorithm for validators
//!
//! Predicts expected block production based on committee composition.
//! Uses the committee size and seats held by our validators to calculate
//! expected blocks per epoch.

/// Block production prediction for an epoch
#[derive(Debug, Clone, Default)]
pub struct BlockPrediction {
    /// Expected number of blocks (based on committee seats)
    pub expected_blocks: f64,
    /// Confidence interval - lower bound (95%)
    pub confidence_low: u64,
    /// Confidence interval - upper bound (95%)
    pub confidence_high: u64,
    /// Actual blocks produced so far
    pub actual_blocks: u64,
    /// Performance ratio (actual / expected), or None if expected is 0
    pub performance_ratio: Option<f64>,
}

/// Block prediction calculator
#[derive(Debug, Clone)]
pub struct PredictionCalculator {
    /// Total slots in an epoch (typically 7200 for Midnight)
    pub epoch_length_slots: u64,
    /// Total committee size (typically ~1200)
    pub committee_size: u64,
}

impl Default for PredictionCalculator {
    fn default() -> Self {
        Self {
            epoch_length_slots: 7200,
            committee_size: 1200,
        }
    }
}

impl PredictionCalculator {
    /// Create a new calculator with custom parameters
    pub fn new(epoch_length_slots: u64, committee_size: u64) -> Self {
        Self {
            epoch_length_slots,
            committee_size,
        }
    }

    /// Calculate expected blocks for a validator with given committee seats
    ///
    /// For AURA consensus:
    /// - Each slot has one designated block producer
    /// - Producer = committee[slot % committee_size]
    /// - Expected blocks = slots_in_epoch * (seats / committee_size)
    ///
    /// # Arguments
    /// * `seats` - Number of seats the validator holds in the committee
    /// * `epoch_progress` - How far into the epoch we are (0.0 to 1.0)
    /// * `actual_blocks` - Blocks actually produced so far
    pub fn calculate(
        &self,
        seats: u64,
        epoch_progress: f64,
        actual_blocks: u64,
    ) -> BlockPrediction {
        if self.committee_size == 0 {
            return BlockPrediction::default();
        }

        // Expected blocks for full epoch
        let expected_full_epoch = (self.epoch_length_slots as f64 * seats as f64)
            / self.committee_size as f64;

        // Expected blocks so far based on epoch progress
        let expected_so_far = expected_full_epoch * epoch_progress;

        // Calculate confidence interval using binomial distribution approximation
        // For each slot, probability of being selected = seats / committee_size
        // Standard deviation = sqrt(n * p * (1-p))
        let slots_so_far = (self.epoch_length_slots as f64 * epoch_progress) as u64;
        let p = seats as f64 / self.committee_size as f64;
        let std_dev = (slots_so_far as f64 * p * (1.0 - p)).sqrt();

        // 95% confidence interval (approximately 2 standard deviations)
        let margin = 2.0 * std_dev;
        let confidence_low = (expected_so_far - margin).max(0.0) as u64;
        let confidence_high = (expected_so_far + margin).ceil() as u64;

        // Calculate performance ratio
        let performance_ratio = if expected_so_far > 0.0 {
            Some(actual_blocks as f64 / expected_so_far)
        } else {
            None
        };

        BlockPrediction {
            expected_blocks: expected_so_far,
            confidence_low,
            confidence_high,
            actual_blocks,
            performance_ratio,
        }
    }

    /// Calculate expected blocks for a full epoch
    pub fn expected_for_full_epoch(&self, seats: u64) -> f64 {
        if self.committee_size == 0 {
            return 0.0;
        }
        (self.epoch_length_slots as f64 * seats as f64) / self.committee_size as f64
    }

    /// Get performance status indicator
    ///
    /// Returns a simple status indicator:
    /// - "excellent" if ratio >= 1.1 (producing more than expected)
    /// - "good" if ratio >= 0.9 (on track)
    /// - "warning" if ratio >= 0.5 (slightly behind)
    /// - "poor" if ratio < 0.5 (significantly behind)
    /// - "unknown" if no expected blocks
    pub fn performance_status(prediction: &BlockPrediction) -> &'static str {
        match prediction.performance_ratio {
            Some(ratio) if ratio >= 1.1 => "excellent",
            Some(ratio) if ratio >= 0.9 => "good",
            Some(ratio) if ratio >= 0.5 => "warning",
            Some(_) => "poor",
            None => "unknown",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prediction_single_seat() {
        let calc = PredictionCalculator::new(7200, 1200);

        // With 1 seat, expect 6 blocks per epoch (7200 / 1200 = 6)
        let prediction = calc.calculate(1, 1.0, 5);

        assert!((prediction.expected_blocks - 6.0).abs() < 0.01);
        assert_eq!(prediction.actual_blocks, 5);
        assert!(prediction.performance_ratio.unwrap() < 1.0);
    }

    #[test]
    fn test_prediction_multiple_seats() {
        let calc = PredictionCalculator::new(7200, 1200);

        // With 10 seats, expect 60 blocks per epoch
        let prediction = calc.calculate(10, 1.0, 58);

        assert!((prediction.expected_blocks - 60.0).abs() < 0.01);
        assert_eq!(prediction.actual_blocks, 58);

        // Should be "good" status (58/60 = 0.967)
        assert_eq!(PredictionCalculator::performance_status(&prediction), "good");
    }

    #[test]
    fn test_prediction_half_epoch() {
        let calc = PredictionCalculator::new(7200, 1200);

        // With 10 seats at 50% through epoch, expect 30 blocks
        let prediction = calc.calculate(10, 0.5, 28);

        assert!((prediction.expected_blocks - 30.0).abs() < 0.01);
        assert!(prediction.confidence_low <= 30);
        assert!(prediction.confidence_high >= 30);
    }

    #[test]
    fn test_prediction_zero_committee() {
        let calc = PredictionCalculator::new(7200, 0);

        let prediction = calc.calculate(10, 1.0, 5);

        assert_eq!(prediction.expected_blocks, 0.0);
        assert!(prediction.performance_ratio.is_none());
    }

    #[test]
    fn test_expected_full_epoch() {
        let calc = PredictionCalculator::new(7200, 1200);

        assert!((calc.expected_for_full_epoch(1) - 6.0).abs() < 0.01);
        assert!((calc.expected_for_full_epoch(20) - 120.0).abs() < 0.01);
    }

    #[test]
    fn test_performance_status() {
        let excellent = BlockPrediction {
            expected_blocks: 10.0,
            confidence_low: 8,
            confidence_high: 12,
            actual_blocks: 12,
            performance_ratio: Some(1.2),
        };
        assert_eq!(PredictionCalculator::performance_status(&excellent), "excellent");

        let good = BlockPrediction {
            performance_ratio: Some(0.95),
            ..excellent.clone()
        };
        assert_eq!(PredictionCalculator::performance_status(&good), "good");

        let warning = BlockPrediction {
            performance_ratio: Some(0.6),
            ..excellent.clone()
        };
        assert_eq!(PredictionCalculator::performance_status(&warning), "warning");

        let poor = BlockPrediction {
            performance_ratio: Some(0.3),
            ..excellent.clone()
        };
        assert_eq!(PredictionCalculator::performance_status(&poor), "poor");
    }
}
