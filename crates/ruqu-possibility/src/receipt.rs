//! Auditable receipts: evidence provenance, verifier passes, and the
//! [`CollapseReceipt`] that records *why* one possibility was selected over the
//! other plausible paths.

use serde::{Deserialize, Serialize};

use crate::gate::GateDecision;

/// A provenance record for a single piece of evidence backing a possibility.
///
/// The `payload_hash` is a BLAKE3 hex digest of the underlying source bytes, so
/// a receipt can be verified later without retaining the (possibly large or
/// sensitive) source payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvidenceReceipt {
    /// Logical source identifier (document id, sensor id, agent id, ...).
    pub source: String,
    /// Trust weight for the source in `[0, 1]`.
    pub trust: f64,
    /// BLAKE3 hex digest of the source payload bytes.
    pub payload_hash: String,
}

impl EvidenceReceipt {
    /// Build an evidence receipt, hashing `payload` with BLAKE3.
    pub fn new(source: impl Into<String>, trust: f64, payload: &[u8]) -> Self {
        Self {
            source: source.into(),
            trust: trust.clamp(0.0, 1.0),
            payload_hash: blake3::hash(payload).to_hex().to_string(),
        }
    }
}

/// The result of an independent verifier pass over a reasoning chain or
/// candidate (e.g. an evidence gate, a contradiction scan, a policy check).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VerifierResult {
    /// Verifier name.
    pub name: String,
    /// Whether the verifier passed.
    pub passed: bool,
    /// Human-readable detail.
    pub detail: String,
}

impl VerifierResult {
    /// Construct a verifier result.
    pub fn new(name: impl Into<String>, passed: bool, detail: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            passed,
            detail: detail.into(),
        }
    }
}

/// A candidate that was *not* selected during collapse, with the reason.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RejectedCandidate {
    /// The rejected candidate's id.
    pub id: String,
    /// Why it lost (low interference probability, contradiction, ...).
    pub reason: String,
}

/// An auditable record of a single collapse: which possibility was selected,
/// which were rejected and why, and the structural metrics at decision time.
///
/// Receipts are content-addressable via [`CollapseReceipt::receipt_hash`] and
/// serialize to stable JSON for governance evidence and replay.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CollapseReceipt {
    /// Deterministic run identifier (derived from the seed and field hash).
    pub run_id: String,
    /// BLAKE3 hex digest of the field that was collapsed.
    pub field_hash: String,
    /// The selected possibility's id.
    pub selected_id: String,
    /// The rejected possibilities and the reason each lost.
    pub rejected: Vec<RejectedCandidate>,
    /// Field phase coherence in `[0, 1]` at decision time.
    pub coherence: f64,
    /// Shannon entropy (bits) of the field before collapse.
    pub entropy_before: f64,
    /// Shannon entropy (bits) of the field after collapse.
    pub entropy_after: f64,
    /// The coherence-gate decision attached to this collapse.
    pub gate_decision: GateDecision,
    /// Independent verifier results, if any were run.
    pub verifier_results: Vec<VerifierResult>,
    /// The RNG seed used for collapse (enables deterministic replay).
    pub seed: u64,
}

impl CollapseReceipt {
    /// Content hash of the receipt: a BLAKE3 digest over the canonical JSON
    /// encoding. Two receipts with identical content produce the same hash.
    pub fn receipt_hash(&self) -> String {
        let json = serde_json::to_vec(self).unwrap_or_default();
        blake3::hash(&json).to_hex().to_string()
    }

    /// Pretty-printed JSON encoding of the receipt.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
}
