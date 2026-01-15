/// Extract AURA slot number from block digest logs
///
/// AURA pre-runtime digest format:
/// - First byte: 0x06 (PreRuntime)
/// - Bytes 1-4: "aura" in hex (61757261)
/// - Bytes 5-12: slot number as little-endian u64
///
/// Example: "0x066175726120778c911100000000"
pub fn extract_slot_from_digest(logs: &[String]) -> Option<u64> {
    for log in logs {
        // PreRuntime AURA format: 0x06 + "aura"(61757261) + slot_le_bytes
        if log.starts_with("0x0661757261") && log.len() >= 30 {
            let slot_hex = &log[14..30]; // 8 bytes = 16 hex chars
            let bytes = hex::decode(slot_hex).ok()?;
            let arr: [u8; 8] = bytes.try_into().ok()?;
            return Some(u64::from_le_bytes(arr));
        }
    }
    None
}

/// Extract timestamp from block extrinsics
///
/// The first extrinsic is typically the timestamp set inherent.
/// Format: compact-encoded call index + compact timestamp
pub fn extract_timestamp_from_extrinsics(extrinsics: &[String]) -> Option<u64> {
    // First extrinsic is usually set_timestamp
    // This is a simplified extraction - full SCALE decoding would be more robust
    if let Some(first) = extrinsics.first() {
        // Skip the first few bytes (length prefix, call index)
        // The timestamp is encoded as a compact u64
        // For now, return None - implement proper SCALE decoding later
        let _ = first;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_slot_from_digest() {
        // Real digest log from Midnight node
        let logs = vec![
            "0x066175726120778c911100000000".to_string(),
            "0x066d637368804404db62c3e40b047c638c2cc3ae2d45678b65b2fc57b748c4d1a9576bf4bc8c"
                .to_string(),
        ];

        let slot = extract_slot_from_digest(&logs);
        assert_eq!(slot, Some(294751351));
    }

    #[test]
    fn test_extract_slot_empty_logs() {
        let logs: Vec<String> = vec![];
        assert_eq!(extract_slot_from_digest(&logs), None);
    }

    #[test]
    fn test_extract_slot_no_aura() {
        let logs = vec!["0x066d637368804404db62c3e40b047c638c".to_string()];
        assert_eq!(extract_slot_from_digest(&logs), None);
    }

    #[test]
    fn test_extract_slot_truncated() {
        // Too short to contain full slot
        let logs = vec!["0x0661757261".to_string()];
        assert_eq!(extract_slot_from_digest(&logs), None);
    }
}
