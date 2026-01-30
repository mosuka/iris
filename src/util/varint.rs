//! Variable-length integer encoding utilities.
//!
//! This module provides efficient variable-length integer encoding and decoding,
//! similar to what's used in protocol buffers and other binary formats.

use crate::error::{IrisError, Result};

/// Encode a u64 value using variable-length encoding.
pub fn encode_u64(value: u64) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut val = value;

    loop {
        let mut byte = (val & 0x7F) as u8;
        val >>= 7;

        if val != 0 {
            byte |= 0x80; // Set continuation bit
        }

        bytes.push(byte);

        if val == 0 {
            break;
        }
    }

    bytes
}

/// Decode a u64 value from variable-length encoding.
pub fn decode_u64(bytes: &[u8]) -> Result<(u64, usize)> {
    let mut result = 0u64;
    let mut shift = 0;
    let mut bytes_read = 0;

    for &byte in bytes {
        bytes_read += 1;

        if shift >= 64 {
            return Err(IrisError::other("VarInt overflow"));
        }

        result |= ((byte & 0x7F) as u64) << shift;

        if (byte & 0x80) == 0 {
            return Ok((result, bytes_read));
        }

        shift += 7;
    }

    Err(IrisError::other("Incomplete VarInt"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_u64() {
        let test_values = [0, 1, 127, 128, 255, 256, 16383, 16384, u64::MAX];

        for &value in &test_values {
            let encoded = encode_u64(value);
            let (decoded, bytes_read) = decode_u64(&encoded).unwrap();

            assert_eq!(value, decoded);
            assert_eq!(encoded.len(), bytes_read);
        }
    }

    #[test]
    fn test_encoding_efficiency() {
        // Large values should use more bytes
        assert!(encode_u64(u64::MAX).len() <= 10);
    }

    #[test]
    fn test_incomplete_varint() {
        // Test with incomplete data (continuation bit set but no more bytes)
        let incomplete = vec![0x80]; // Continuation bit set but no more data
        assert!(decode_u64(&incomplete).is_err());
    }

    #[test]
    fn test_overflow() {
        // Test with data that would overflow u64 (more than 10 bytes usually, or just massive bytes)
        let overflow_data = vec![0xFF; 20]; // Too many bytes for u64
        let result = decode_u64(&overflow_data);
        assert!(result.is_err());
    }
}
