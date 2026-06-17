//! Coherence gate: converts structural risk into `PERMIT`, `DEFER`, or `DENY`.
//!
//! This is the lightweight, dependency-free gate that operates directly on a
//! [`PossibilityField`](crate::PossibilityField)'s coherence and entropy. It is
//! intentionally *not* the full `ruqu` min-cut fabric gate (which requires the
//! `ruvector-mincut` substrate); rather it is the decision primitive the
//! possibility runtime uses to decide whether a collapse is safe to act on.

use serde::{Deserialize, Serialize};

use crate::field::PossibilityField;

/// The three-valued structural decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GateDecision {
    /// Evidence is structurally coherent — execute normally.
    Permit,
    /// Uncertainty remains — use a slower path, more retrieval, or human review.
    Defer,
    /// Structurally unstable — block the action and quarantine state.
    Deny,
}

impl GateDecision {
    /// Uppercase string form (`"PERMIT" | "DEFER" | "DENY"`).
    pub fn as_str(&self) -> &'static str {
        match self {
            GateDecision::Permit => "PERMIT",
            GateDecision::Defer => "DEFER",
            GateDecision::Deny => "DENY",
        }
    }

    /// True if the decision permits action.
    pub fn is_permit(&self) -> bool {
        matches!(self, GateDecision::Permit)
    }

    /// True if the decision blocks action.
    pub fn is_deny(&self) -> bool {
        matches!(self, GateDecision::Deny)
    }
}

/// Thresholds that map a field's structural metrics onto a [`GateDecision`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct GateThresholds {
    /// Coherence at or above this value is permit-eligible.
    pub permit_coherence: f64,
    /// Coherence below this value forces `DENY`.
    pub deny_coherence: f64,
    /// Entropy (bits) above this value downgrades a `PERMIT` to `DEFER`
    /// (the entropy floor / "uncertainty remains" rule).
    pub max_entropy: f64,
}

impl Default for GateThresholds {
    fn default() -> Self {
        Self {
            permit_coherence: 0.70,
            deny_coherence: 0.30,
            max_entropy: 2.0,
        }
    }
}

/// A coherence gate parameterized by [`GateThresholds`].
#[derive(Debug, Clone, Copy, Default)]
pub struct CoherenceGate {
    /// The thresholds applied during [`CoherenceGate::evaluate`].
    pub thresholds: GateThresholds,
}

impl CoherenceGate {
    /// Construct a gate with explicit thresholds.
    pub fn new(thresholds: GateThresholds) -> Self {
        Self { thresholds }
    }

    /// Construct a gate with default thresholds.
    pub fn with_defaults() -> Self {
        Self::default()
    }

    /// Evaluate a field and return a structural decision.
    ///
    /// Rules, in order:
    /// 1. coherence `< deny_coherence` → `DENY`
    /// 2. coherence `>= permit_coherence` **and** entropy `<= max_entropy` → `PERMIT`
    /// 3. otherwise → `DEFER`
    pub fn evaluate<T>(&self, field: &PossibilityField<T>) -> GateDecision {
        let coherence = field.coherence();
        let entropy = field.entropy();
        self.evaluate_metrics(coherence, entropy)
    }

    /// Evaluate raw coherence/entropy metrics (useful when the metrics are
    /// computed elsewhere, e.g. after interference scoring).
    pub fn evaluate_metrics(&self, coherence: f64, entropy: f64) -> GateDecision {
        let t = &self.thresholds;
        if coherence < t.deny_coherence {
            GateDecision::Deny
        } else if coherence >= t.permit_coherence && entropy <= t.max_entropy {
            GateDecision::Permit
        } else {
            GateDecision::Defer
        }
    }
}
