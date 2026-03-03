//! Common ID generation and parsing for Shard-Prefixed Stable IDs.
//!
//! Bits 48-63: Shard ID (16 bits)
//! Bits 0-47:  Local ID (48 bits)

/// Number of bits allocated to the local (per-shard) document ID within a 64-bit
/// composite document ID. The remaining upper bits (64 - 48 = 16) store the shard ID.
pub const LOCAL_ID_BITS: u32 = 48;

/// Bitmask that isolates the local document ID from a 64-bit composite document ID.
///
/// Equal to `(1 << LOCAL_ID_BITS) - 1`, i.e. the lower 48 bits set to 1.
pub const LOCAL_ID_MASK: u64 = (1 << LOCAL_ID_BITS) - 1;

/// Create a 64-bit composite document ID from a shard ID and a local ID.
///
/// The resulting ID packs the `shard_id` into the upper 16 bits and the
/// `local_id` into the lower 48 bits. If `local_id` exceeds
/// [`LOCAL_ID_MASK`] (2^48 - 1), the excess upper bits are silently
/// truncated by a bitwise AND with `LOCAL_ID_MASK`.
///
/// # Arguments
///
/// * `shard_id` - Shard identifier (16-bit).
/// * `local_id` - Per-shard document ID. Values larger than 2^48 - 1 are
///   silently truncated to 48 bits.
///
/// # Returns
///
/// A 64-bit composite document ID.
pub fn create_doc_id(shard_id: u16, local_id: u64) -> u64 {
    ((shard_id as u64) << LOCAL_ID_BITS) | (local_id & LOCAL_ID_MASK)
}

/// Extract the local ID from a 64-bit doc ID.
pub fn get_local_id(doc_id: u64) -> u64 {
    doc_id & LOCAL_ID_MASK
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_id_roundtrip() {
        let shard_id = 42u16;
        let local_id = 12345u64;
        let doc_id = create_doc_id(shard_id, local_id);

        assert_eq!(get_local_id(doc_id), local_id);
    }

    #[test]
    fn test_max_local_id() {
        let shard_id = 1u16;
        let local_id = LOCAL_ID_MASK;
        let doc_id = create_doc_id(shard_id, local_id);

        assert_eq!(get_local_id(doc_id), local_id);
    }
}
