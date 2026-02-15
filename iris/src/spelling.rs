//! Spelling correction and suggestion utilities for Iris.
//!
//! This module powers typo tolerance across the lexical pipeline by providing
//! dictionary builders, edit-distance based correction, and "Did you mean?"
//! suggestion helpers that can be surfaced in UI flows or auto-correct logic.

pub mod corrector;
pub mod dictionary;
pub mod suggest;
pub mod typo_patterns;
