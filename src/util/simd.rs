//! SIMD-friendly helpers that power hot paths inside Iris.

/// SIMD-accelerated ASCII operations.
pub mod ascii {

    /// Convert ASCII characters to lowercase using optimized byte operations.
    ///
    /// This function processes bytes in chunks for better performance
    /// while maintaining the benefits of SIMD-style thinking.
    pub fn to_lowercase_optimized(input: &str) -> String {
        let bytes = input.as_bytes();
        let mut result = Vec::with_capacity(bytes.len());

        // Process 8 bytes at a time for better cache efficiency
        let chunks = bytes.chunks_exact(8);
        let remainder = chunks.remainder();

        for chunk in chunks {
            let mut processed = [0u8; 8];
            for (i, &byte) in chunk.iter().enumerate() {
                // ASCII uppercase A-Z to lowercase conversion
                if byte.is_ascii_uppercase() {
                    processed[i] = byte + 32; // Convert to lowercase
                } else {
                    processed[i] = byte;
                }
            }
            result.extend_from_slice(&processed);
        }

        // Handle remaining bytes
        for &byte in remainder {
            if byte.is_ascii_uppercase() {
                result.push(byte + 32);
            } else {
                result.push(byte);
            }
        }

        // Convert back to string (we know it's valid UTF-8 since input was ASCII)
        unsafe { String::from_utf8_unchecked(result) }
    }

    /// Fallback implementation for non-ASCII or when optimization is not beneficial.
    pub fn to_lowercase_fallback(input: &str) -> String {
        input.to_lowercase()
    }

    /// Main entry point for optimized lowercase conversion.
    ///
    /// This function automatically chooses between optimized and fallback
    /// implementations based on the input characteristics.
    pub fn to_lowercase(input: &str) -> String {
        // Check if input is ASCII for optimization
        if input.is_ascii() && input.len() >= 16 {
            to_lowercase_optimized(input)
        } else {
            to_lowercase_fallback(input)
        }
    }

    /// Find the first whitespace character position using optimized search.
    pub fn find_whitespace_optimized(input: &[u8]) -> Option<usize> {
        if input.len() < 8 {
            return input.iter().position(|&b| b.is_ascii_whitespace());
        }

        let mut chunks = input.chunks_exact(8);
        let remainder = chunks.remainder();
        let mut chunk_idx = 0;

        for chunk in &mut chunks {
            for (byte_idx, &byte) in chunk.iter().enumerate() {
                if byte == b' ' || byte == b'\t' || byte == b'\n' || byte == b'\r' {
                    return Some(chunk_idx * 8 + byte_idx);
                }
            }
            chunk_idx += 1;
        }

        // Check remainder
        let base_offset = chunk_idx * 8;
        remainder
            .iter()
            .position(|&b| b.is_ascii_whitespace())
            .map(|pos| base_offset + pos)
    }

    /// Optimized whitespace finding (public API).
    pub fn find_whitespace_simd(input: &[u8]) -> Option<usize> {
        find_whitespace_optimized(input)
    }
}

/// SIMD-accelerated numerical operations for scoring.
pub mod numeric {

    /// Batch BM25 score calculation for multiple documents.
    ///
    /// This function processes multiple TF values simultaneously
    /// for better performance in scoring operations.
    pub fn batch_bm25_tf(term_freqs: &[f32], k1: f32, norm_factors: &[f32]) -> Vec<f32> {
        assert_eq!(term_freqs.len(), norm_factors.len());

        let mut results = Vec::with_capacity(term_freqs.len());

        // Process 4 values at a time for better performance
        let chunks = term_freqs.chunks_exact(4);
        let remainder = chunks.remainder();
        let norm_chunks = norm_factors.chunks_exact(4);

        for (tf_chunk, norm_chunk) in chunks.zip(norm_chunks) {
            let mut batch_results = [0.0f32; 4];

            for i in 0..4 {
                let tf = tf_chunk[i];
                let norm = norm_chunk[i];

                // BM25 TF calculation: tf * (k1 + 1) / (tf + k1 * norm)
                batch_results[i] = (tf * (k1 + 1.0)) / (tf + k1 * norm);
            }

            results.extend_from_slice(&batch_results);
        }

        // Handle remaining values
        let norm_remainder = &norm_factors[norm_factors.len() - remainder.len()..];
        for (tf, norm) in remainder.iter().zip(norm_remainder.iter()) {
            let tf_score = (tf * (k1 + 1.0)) / (tf + k1 * norm);
            results.push(tf_score);
        }

        results
    }

    /// Batch final BM25 score calculation.
    pub fn batch_bm25_final_score(
        tf_scores: &[f32],
        idf_scores: &[f32],
        boosts: &[f32],
    ) -> Vec<f32> {
        assert_eq!(tf_scores.len(), idf_scores.len());
        assert_eq!(tf_scores.len(), boosts.len());

        let mut results = Vec::with_capacity(tf_scores.len());

        // Process 4 values at a time
        let chunks = tf_scores.chunks_exact(4);
        let remainder = chunks.remainder();
        let idf_chunks = idf_scores.chunks_exact(4);
        let boost_chunks = boosts.chunks_exact(4);

        for ((tf_chunk, idf_chunk), boost_chunk) in chunks.zip(idf_chunks).zip(boost_chunks) {
            let mut batch_results = [0.0f32; 4];

            for i in 0..4 {
                // Final BM25: IDF * TF * boost
                batch_results[i] = idf_chunk[i] * tf_chunk[i] * boost_chunk[i];
            }

            results.extend_from_slice(&batch_results);
        }

        // Handle remaining values
        let idf_remainder = &idf_scores[idf_scores.len() - remainder.len()..];
        let boost_remainder = &boosts[boosts.len() - remainder.len()..];

        for ((tf, idf), boost) in remainder
            .iter()
            .zip(idf_remainder.iter())
            .zip(boost_remainder.iter())
        {
            results.push(idf * tf * boost);
        }

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimized_lowercase_ascii() {
        let input = "HELLO WORLD THIS IS A TEST STRING FOR OPTIMIZATION";
        let expected = "hello world this is a test string for optimization";
        let result = ascii::to_lowercase(input);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_optimized_lowercase_mixed() {
        let input = "Hello World 123 ABC def";
        let expected = "hello world 123 abc def";
        let result = ascii::to_lowercase(input);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_optimized_lowercase_empty() {
        let result = ascii::to_lowercase("");
        assert_eq!(result, "");
    }

    #[test]
    fn test_optimized_lowercase_short() {
        let input = "ABC";
        let expected = "abc";
        let result = ascii::to_lowercase(input);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_fallback_for_unicode() {
        let input = "Héllo Wörld"; // Non-ASCII characters
        let result = ascii::to_lowercase(input);
        let expected = input.to_lowercase();
        assert_eq!(result, expected);
    }
}
