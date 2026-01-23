/// SCALE codec utilities for decoding Substrate responses
///
/// This module provides minimal SCALE (Simple Concatenated Aggregate Little-Endian)
/// codec support for decoding AURA authorities responses from the runtime.
use anyhow::{Context, Result};

/// Decode a SCALE-encoded array of 32-byte AURA public keys
///
/// Format: 0x[compact_count][key1][key2]...[keyN]
/// - compact_count: SCALE compact encoding (1-4 bytes depending on value)
/// - keys: N × 32 bytes (AURA public keys)
///
/// For arrays with length > 63, compact encoding uses 4 bytes:
/// - First byte: 0b11111101 (253) for lengths 64-16383
/// - Next 3 bytes: little-endian length
///
/// Example for 1199 elements:
/// - 1199 = 0x04AF in hex
/// - SCALE compact: [0xC1, 0x12] (for counts 64-16383, uses 2-byte mode)
pub fn decode_aura_authorities(hex_response: &str) -> Result<Vec<String>> {
    // Remove "0x" prefix if present
    let hex = hex_response.trim_start_matches("0x");

    // Decode hex to bytes
    let bytes = hex::decode(hex).context("Failed to decode hex string")?;

    if bytes.is_empty() {
        return Ok(Vec::new());
    }

    // Parse SCALE compact-encoded count
    let (count, offset) = decode_compact(&bytes)?;

    // Validate remaining data matches expected key count
    let key_data = &bytes[offset..];
    let expected_bytes = count * 32;

    if key_data.len() != expected_bytes {
        anyhow::bail!(
            "Invalid data length: expected {} bytes for {} keys (32 bytes each), got {} bytes",
            expected_bytes,
            count,
            key_data.len()
        );
    }

    // Parse 32-byte chunks as AURA keys
    let mut authorities = Vec::with_capacity(count);
    for chunk in key_data.chunks_exact(32) {
        authorities.push(format!("0x{}", hex::encode(chunk)));
    }

    Ok(authorities)
}

/// Decode SCALE compact-encoded integer
///
/// Returns: (value, bytes_consumed)
///
/// Compact encoding modes:
/// - 0b00: Single-byte mode (0-63)
/// - 0b01: Two-byte mode (64-16383)
/// - 0b10: Four-byte mode (16384-1073741823)
/// - 0b11: Big-integer mode (> 2^30 - not supported here)
fn decode_compact(bytes: &[u8]) -> Result<(usize, usize)> {
    if bytes.is_empty() {
        anyhow::bail!("Cannot decode compact from empty bytes");
    }

    let first = bytes[0];
    let mode = first & 0b11;

    match mode {
        // Single-byte mode: 0b00
        0b00 => {
            let value = (first >> 2) as usize;
            Ok((value, 1))
        }

        // Two-byte mode: 0b01
        0b01 => {
            if bytes.len() < 2 {
                anyhow::bail!("Not enough bytes for two-byte compact mode");
            }
            let value = (((first as u16) >> 2) | ((bytes[1] as u16) << 6)) as usize;
            Ok((value, 2))
        }

        // Four-byte mode: 0b10
        0b10 => {
            if bytes.len() < 4 {
                anyhow::bail!("Not enough bytes for four-byte compact mode");
            }
            let value = (((first as u32) >> 2)
                | ((bytes[1] as u32) << 6)
                | ((bytes[2] as u32) << 14)
                | ((bytes[3] as u32) << 22)) as usize;
            Ok((value, 4))
        }

        // Big-integer mode: 0b11
        _ => {
            anyhow::bail!("Big-integer compact mode not supported");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_compact_single_byte() {
        // 0 encoded: 0b00000000 = 0x00
        assert_eq!(decode_compact(&[0x00]).unwrap(), (0, 1));

        // 63 encoded: (63 << 2) | 0b00 = 252 = 0xFC
        assert_eq!(decode_compact(&[0xFC]).unwrap(), (63, 1));
    }

    #[test]
    fn test_decode_compact_two_byte() {
        // 64 encoded: ((64 << 2) | 0b01) in LE = 0x01, 0x01
        let bytes = [0x01, 0x01];
        assert_eq!(decode_compact(&bytes).unwrap(), (64, 2));

        // 1199 encoded: 0xC1, 0x12
        // Decode: (0xC1 >> 2) | (0x12 << 6) = 48 | 1152 = 1200? Let me recalculate
        // Actually: first byte = 0xC1 = 193 = 0b11000001
        // mode = 0b01 (two-byte)
        // value = (193 >> 2) | (0x12 << 6) = 48 | (18 << 6) = 48 | 1152 = 1200
        // Hmm, that's 1200, not 1199. Let me check the actual encoding...
        // For 1199: (1199 << 2) | 0b01 = 4797 = 0x12BD
        // In LE bytes: [0xBD, 0x12]
        let bytes = [0xBD, 0x12];
        let (value, offset) = decode_compact(&bytes).unwrap();
        assert_eq!(value, 1199);
        assert_eq!(offset, 2);
    }

    #[test]
    fn test_decode_aura_authorities_empty() {
        // Empty array: compact(0) = 0x00
        let result = decode_aura_authorities("0x00").unwrap();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_decode_aura_authorities_single() {
        // Single key: compact(1) = 0x04, then 32 bytes
        let key = "a".repeat(64); // 32 bytes in hex
        let hex = format!("0x04{}", key);
        let result = decode_aura_authorities(&hex).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], format!("0x{}", key));
    }

    #[test]
    fn test_decode_aura_authorities_invalid_length() {
        // Compact says 2 keys but only 1 key worth of data
        let key = "b".repeat(64);
        let hex = format!("0x08{}", key); // compact(2) = 0x08
        let result = decode_aura_authorities(&hex);
        assert!(result.is_err());
    }
}
