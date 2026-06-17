//! Error types for the possibility runtime.

use thiserror::Error;

/// Errors that can arise while constructing, scoring, or collapsing a
/// [`PossibilityField`](crate::PossibilityField).
#[derive(Debug, Error)]
pub enum PossibilityError {
    /// A collapse or scoring operation was attempted on a field with no
    /// candidates.
    #[error("possibility field is empty")]
    EmptyField,

    /// An amplitude was negative or non-finite.
    #[error("invalid amplitude: {0}")]
    InvalidAmplitude(f64),

    /// JSON (de)serialization failure.
    #[error("serialization error: {0}")]
    Serde(String),
}

/// Convenience result alias for the possibility runtime.
pub type Result<T> = std::result::Result<T, PossibilityError>;
