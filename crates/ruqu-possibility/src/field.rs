//! The possibility field: a set of competing hypotheses, each carrying a
//! complex amplitude (`amplitude` magnitude + `phase`), plus the structural
//! metrics (entropy, coherence) and the collapse operation that selects one.

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};

use crate::error::{PossibilityError, Result};
use crate::gate::CoherenceGate;
use crate::receipt::{CollapseReceipt, EvidenceReceipt, RejectedCandidate};

/// A single competing hypothesis in a [`PossibilityField`].
///
/// The amplitude is stored in polar form: `amplitude` is the (non-negative)
/// magnitude and `phase` is the angle in radians. The selection probability of
/// a possibility is proportional to `amplitude²`; the `phase` governs how it
/// interferes with the rest of the field (constructive vs destructive).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Possibility<T> {
    /// Stable identifier for this possibility.
    pub id: String,
    /// The carried hypothesis (retrieval chunk, plan, interpretation, design...).
    pub payload: T,
    /// Non-negative amplitude magnitude.
    pub amplitude: f64,
    /// Phase angle in radians.
    pub phase: f64,
    /// Evidence backing this possibility.
    pub evidence: Vec<EvidenceReceipt>,
}

impl<T> Possibility<T> {
    /// Create a possibility. Negative or non-finite amplitudes are clamped to 0.
    pub fn new(id: impl Into<String>, payload: T, amplitude: f64, phase: f64) -> Self {
        let amplitude = if amplitude.is_finite() {
            amplitude.max(0.0)
        } else {
            0.0
        };
        Self {
            id: id.into(),
            payload,
            amplitude,
            phase,
            evidence: Vec::new(),
        }
    }

    /// Attach evidence to this possibility (builder style).
    pub fn with_evidence(mut self, evidence: Vec<EvidenceReceipt>) -> Self {
        self.evidence = evidence;
        self
    }

    /// Unnormalized selection weight, `amplitude²`.
    pub fn probability(&self) -> f64 {
        self.amplitude * self.amplitude
    }

    /// Real part of the complex amplitude.
    pub(crate) fn re(&self) -> f64 {
        self.amplitude * self.phase.cos()
    }

    /// Imaginary part of the complex amplitude.
    pub(crate) fn im(&self) -> f64 {
        self.amplitude * self.phase.sin()
    }
}

/// A field of competing possibilities.
///
/// The field exposes the structural quantities the runtime reasons over:
/// [`PossibilityField::entropy`] (how spread the distribution is),
/// [`PossibilityField::coherence`] (how phase-aligned the field is), and
/// [`PossibilityField::collapse`] (deterministic, seeded selection that emits a
/// [`CollapseReceipt`]).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PossibilityField<T> {
    /// The competing possibilities.
    pub candidates: Vec<Possibility<T>>,
    /// Coherence threshold below which the runtime should refuse a hard collapse
    /// (used by callers / gates; collapse itself is always seed-deterministic).
    pub collapse_threshold: f64,
}

impl<T> PossibilityField<T> {
    /// Build a field from candidates with a default (zero) collapse threshold.
    pub fn new(candidates: Vec<Possibility<T>>) -> Self {
        Self {
            candidates,
            collapse_threshold: 0.0,
        }
    }

    /// Set the collapse threshold (builder style).
    pub fn with_threshold(mut self, threshold: f64) -> Self {
        self.collapse_threshold = threshold;
        self
    }

    /// Number of candidates.
    pub fn len(&self) -> usize {
        self.candidates.len()
    }

    /// Whether the field has no candidates.
    pub fn is_empty(&self) -> bool {
        self.candidates.is_empty()
    }

    /// Total power, `Σ amplitudeₖ²`.
    pub fn total_power(&self) -> f64 {
        self.candidates.iter().map(Possibility::probability).sum()
    }

    /// Normalized probability distribution over candidates (`Σ p = 1`). Returns
    /// an empty vector for an empty or zero-power field.
    pub fn probabilities(&self) -> Vec<f64> {
        let total = self.total_power();
        if total <= f64::EPSILON {
            return vec![0.0; self.candidates.len()];
        }
        self.candidates
            .iter()
            .map(|c| c.probability() / total)
            .collect()
    }

    /// Shannon entropy of the normalized distribution, in **bits**.
    ///
    /// `0` for a fully collapsed field (one dominant candidate); `log2(n)` for a
    /// uniform field of `n` candidates.
    pub fn entropy(&self) -> f64 {
        let probs = self.probabilities();
        -probs
            .iter()
            .filter(|&&p| p > 0.0)
            .map(|&p| p * p.log2())
            .sum::<f64>()
    }

    /// Phase coherence of the field in `[0, 1]`.
    ///
    /// Defined as `|Σ aₖ e^{iφₖ}|² / (Σ aₖ)²`. It is `1.0` when every
    /// possibility shares the same phase (fully constructive) and approaches `0`
    /// when phases cancel (destructive / contradictory field).
    pub fn coherence(&self) -> f64 {
        let sum_amp: f64 = self.candidates.iter().map(|c| c.amplitude).sum();
        if sum_amp <= 1e-15 {
            return 0.0;
        }
        let re: f64 = self.candidates.iter().map(Possibility::re).sum();
        let im: f64 = self.candidates.iter().map(Possibility::im).sum();
        ((re * re + im * im) / (sum_amp * sum_amp)).clamp(0.0, 1.0)
    }

    /// The candidate with the largest selection probability, if any.
    pub fn argmax(&self) -> Option<&Possibility<T>> {
        self.candidates.iter().max_by(|a, b| {
            a.probability()
                .partial_cmp(&b.probability())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    /// Scale all amplitudes so that `Σ amplitudeₖ² = 1`. No-op for a zero-power
    /// field.
    pub fn normalize(&mut self) {
        let total = self.total_power();
        if total <= f64::EPSILON {
            return;
        }
        let norm = total.sqrt();
        for c in &mut self.candidates {
            c.amplitude /= norm;
        }
    }

    /// Content hash of the field: a BLAKE3 digest over each candidate's id,
    /// amplitude, and phase (in order) plus the collapse threshold. Stable
    /// across runs and independent of payload type.
    pub fn field_hash(&self) -> String {
        let mut hasher = blake3::Hasher::new();
        for c in &self.candidates {
            hasher.update(c.id.as_bytes());
            hasher.update(&c.amplitude.to_le_bytes());
            hasher.update(&c.phase.to_le_bytes());
        }
        hasher.update(&self.collapse_threshold.to_le_bytes());
        hasher.finalize().to_hex().to_string()
    }
}

impl<T: Clone> PossibilityField<T> {
    /// Deterministically collapse the field to a single possibility using a
    /// seeded weighted draw over the normalized distribution, returning the
    /// selected possibility and a [`CollapseReceipt`].
    ///
    /// The same `(field, seed)` always yields the same selection and the same
    /// coherence — this is the basis for the replay guarantee. The receipt's
    /// gate decision is computed with a default [`CoherenceGate`].
    pub fn collapse(&self, seed: u64) -> Result<(Possibility<T>, CollapseReceipt)> {
        self.collapse_with_gate(seed, &CoherenceGate::with_defaults())
    }

    /// Collapse using a seeded weighted draw with an explicit gate for the
    /// receipt's decision.
    pub fn collapse_with_gate(
        &self,
        seed: u64,
        gate: &CoherenceGate,
    ) -> Result<(Possibility<T>, CollapseReceipt)> {
        if self.candidates.is_empty() {
            return Err(PossibilityError::EmptyField);
        }
        let probs = self.probabilities();
        let selected_idx = Self::sample_index(&probs, seed);
        Ok(self.finish_collapse(selected_idx, &probs, gate, seed))
    }

    /// Deterministically collapse to the **highest-probability** possibility
    /// (the argmax), tie-broken by lowest index, returning a [`CollapseReceipt`].
    ///
    /// Unlike [`collapse`](Self::collapse), the selection does not depend on the
    /// seed — only the receipt's recorded `seed` field does (so a replay can
    /// reconstruct the same field). This is the right mode when the field has
    /// already been scored/ranked (e.g. after interference reranking) and the
    /// caller wants the top result, deterministically, with auditable metrics
    /// computed over the field *in its natural order* (so `field_hash` matches a
    /// replayer who reconstructs the same field).
    pub fn collapse_argmax(&self, seed: u64) -> Result<(Possibility<T>, CollapseReceipt)> {
        self.collapse_argmax_with_gate(seed, &CoherenceGate::with_defaults())
    }

    /// Argmax collapse with an explicit gate for the receipt's decision.
    pub fn collapse_argmax_with_gate(
        &self,
        seed: u64,
        gate: &CoherenceGate,
    ) -> Result<(Possibility<T>, CollapseReceipt)> {
        if self.candidates.is_empty() {
            return Err(PossibilityError::EmptyField);
        }
        let probs = self.probabilities();
        let selected_idx = Self::argmax_index(&probs);
        Ok(self.finish_collapse(selected_idx, &probs, gate, seed))
    }

    /// Seeded weighted draw over a normalized probability distribution.
    fn sample_index(probs: &[f64], seed: u64) -> usize {
        let mut rng = StdRng::seed_from_u64(seed);
        let r: f64 = rng.gen::<f64>();
        let mut cumulative = 0.0;
        for (i, &p) in probs.iter().enumerate() {
            cumulative += p;
            if r <= cumulative {
                return i;
            }
        }
        probs.len() - 1
    }

    /// Index of the maximum probability, tie-broken by lowest index.
    fn argmax_index(probs: &[f64]) -> usize {
        let mut best = 0;
        for i in 1..probs.len() {
            if probs[i] > probs[best] {
                best = i;
            }
        }
        best
    }

    /// Build the selected possibility, the rejected list, and the receipt for a
    /// chosen `selected_idx`. Shared by every collapse mode so they emit
    /// identical receipt structure.
    fn finish_collapse(
        &self,
        selected_idx: usize,
        probs: &[f64],
        gate: &CoherenceGate,
        seed: u64,
    ) -> (Possibility<T>, CollapseReceipt) {
        let entropy_before = self.entropy();
        let coherence = self.coherence();
        let gate_decision = gate.evaluate(self);

        let selected = self.candidates[selected_idx].clone();
        let selected_prob = probs.get(selected_idx).copied().unwrap_or(0.0);

        let rejected = self
            .candidates
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != selected_idx)
            .map(|(i, c)| {
                let p = probs.get(i).copied().unwrap_or(0.0);
                let reason = if p + 1e-12 < selected_prob {
                    format!("lower interference probability ({p:.4} < {selected_prob:.4})")
                } else {
                    format!("not selected by collapse (probability {p:.4})")
                };
                RejectedCandidate {
                    id: c.id.clone(),
                    reason,
                }
            })
            .collect();

        let field_hash = self.field_hash();
        let run_id = format!("run_{:016x}_{}", seed, &field_hash[..8.min(field_hash.len())]);

        let receipt = CollapseReceipt {
            run_id,
            field_hash,
            selected_id: selected.id.clone(),
            rejected,
            coherence,
            entropy_before,
            // A hard collapse yields a pure (delta) distribution: zero entropy.
            entropy_after: 0.0,
            gate_decision,
            verifier_results: Vec::new(),
            seed,
        };

        (selected, receipt)
    }
}
