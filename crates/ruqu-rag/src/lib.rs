//! # ruqu-rag — Interference Reranking for Retrieval-Augmented Generation
//!
//! Phase 2 of ADR-258 ("Interference Reranking"). This crate replaces plain
//! cosine reranking with a **possibility-field interference reranker** built on
//! [`ruqu_possibility`].
//!
//! ## Why interference?
//!
//! Plain cosine retrieval ranks a candidate purely by how close its embedding
//! sits to the query. That is blind to *structure*: an outdated, contradicted,
//! or low-trust chunk that happens to be semantically close to the query is
//! ranked just as highly as a fresh, well-cited, coherent chunk.
//!
//! Interference reranking maps each candidate to a **complex amplitude**:
//!
//! * the **magnitude** encodes how strongly the candidate supports an answer —
//!   cosine similarity gated by source trust, recency, and graph proximity;
//! * the **phase** encodes *which* answer it supports — contradicted candidates
//!   are rotated by `π` so they sit opposite the coherent bulk.
//!
//! When the field interferes, candidates that point the same direction
//! **constructively amplify** one another, while contradicted candidates
//! **destructively cancel**. The result is a reranking where coherent evidence
//! wins even when a contradictory chunk has a higher raw cosine score, and where
//! the whole decision is auditable through a [`CollapseReceipt`].
//!
//! ## Amplitude / phase mapping
//!
//! For query `q` and candidate `c`:
//!
//! ```text
//! sim         = clamp(cosine_similarity(q, c.embedding), 0, 1)
//! magnitude_0 = sim * c.source_trust * c.recency * c.graph_proximity
//! phase_0     = π * c.contradiction               (+ small novelty kick, optional)
//! ```
//!
//! The field is then refined for `interference_rounds` iterations: each
//! candidate's magnitude is rescaled by how well its complex amplitude **aligns
//! with the field's net (resultant) amplitude** — aligned (constructive)
//! candidates are amplified, opposed (destructive) candidates decay. This is the
//! "interference scoring" primitive of ADR-258 applied at the field level, and
//! it is fully deterministic.
//!
//! ## Example
//!
//! ```
//! use ruqu_rag::{QuantumRagIndex, RagCandidate};
//!
//! let mut index = QuantumRagIndex::new(3).interference_rounds(3);
//! // Two coherent, well-cited chunks support the answer ...
//! index.add(RagCandidate::new("good1", "fresh, cited", vec![1.0, 0.0, 0.0]));
//! index.add(RagCandidate::new("good2", "also cited", vec![1.0, 0.0, 0.0]));
//! // ... and one outdated chunk contradicts it (identical cosine).
//! index.add(
//!     RagCandidate::new("bad", "outdated, contradicted", vec![1.0, 0.0, 0.0])
//!         .with_contradiction(1.0),
//! );
//!
//! let result = index.search(&[1.0, 0.0, 0.0], 3, 42).unwrap();
//! // The contradicted candidate is suppressed even though its cosine is identical;
//! // it ends up ranked last.
//! assert_eq!(result.selected.last().unwrap().id, "bad");
//! ```

use ruqu_core::types::Complex;
use ruqu_possibility::{
    CoherenceGate, CollapseReceipt, EvidenceReceipt, GateDecision, Possibility, PossibilityField,
};
use serde::{Deserialize, Serialize};

/// Cosine similarity between two vectors.
///
/// Returns `0.0` for empty vectors or when either vector has zero norm. When the
/// lengths differ, the shorter length is used (extra trailing components are
/// ignored). The result is clamped to `[-1, 1]`.
pub fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let len = a.len().min(b.len());
    let mut dot = 0.0_f64;
    let mut norm_a = 0.0_f64;
    let mut norm_b = 0.0_f64;
    for i in 0..len {
        dot += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }
    let denom = norm_a.sqrt() * norm_b.sqrt();
    if denom < 1e-15 {
        0.0
    } else {
        (dot / denom).clamp(-1.0, 1.0)
    }
}

/// A retrievable candidate chunk plus the structural signals the reranker uses.
///
/// Construct with [`RagCandidate::new`] (sensible defaults) and override the
/// structural fields with the `with_*` builders.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RagCandidate {
    /// Stable identifier (document/chunk id).
    pub id: String,
    /// The chunk text (carried through to the answer / citation).
    pub text: String,
    /// Dense embedding of the chunk.
    pub embedding: Vec<f64>,
    /// Trust weight of the source in `[0, 1]` (1.0 = fully trusted).
    pub source_trust: f64,
    /// Recency weight in `[0, 1]` (1.0 = freshest, 0.0 = stale).
    pub recency: f64,
    /// Knowledge-graph proximity to the query intent in `[0, 1]` (1.0 = direct).
    pub graph_proximity: f64,
    /// Contradiction score in `[0, 1]`: how strongly the chunk *opposes* the
    /// coherent answer. Drives the phase toward `π` (destructive interference).
    pub contradiction: f64,
    /// Novelty score in `[0, 1]`: introduces a small phase perturbation when
    /// phase kickback is enabled, so near-duplicate evidence does not perfectly
    /// stack while genuinely novel evidence is nudged apart.
    pub novelty: f64,
}

impl RagCandidate {
    /// Create a candidate with default structural signals: `source_trust`,
    /// `recency`, `graph_proximity` = `1.0`; `contradiction`, `novelty` = `0.0`.
    pub fn new(id: impl Into<String>, text: impl Into<String>, embedding: Vec<f64>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            embedding,
            source_trust: 1.0,
            recency: 1.0,
            graph_proximity: 1.0,
            contradiction: 0.0,
            novelty: 0.0,
        }
    }

    /// Set the source trust weight (clamped to `[0, 1]`).
    pub fn with_source_trust(mut self, v: f64) -> Self {
        self.source_trust = v.clamp(0.0, 1.0);
        self
    }

    /// Set the recency weight (clamped to `[0, 1]`).
    pub fn with_recency(mut self, v: f64) -> Self {
        self.recency = v.clamp(0.0, 1.0);
        self
    }

    /// Set the graph-proximity weight (clamped to `[0, 1]`).
    pub fn with_graph_proximity(mut self, v: f64) -> Self {
        self.graph_proximity = v.clamp(0.0, 1.0);
        self
    }

    /// Set the contradiction score (clamped to `[0, 1]`).
    pub fn with_contradiction(mut self, v: f64) -> Self {
        self.contradiction = v.clamp(0.0, 1.0);
        self
    }

    /// Set the novelty score (clamped to `[0, 1]`).
    pub fn with_novelty(mut self, v: f64) -> Self {
        self.novelty = v.clamp(0.0, 1.0);
        self
    }

    /// Non-negative magnitude combiner: cosine (clamped to `[0, 1]`) gated by the
    /// multiplicative trust / recency / proximity weights. Monotone in every
    /// argument and always `>= 0`.
    fn base_magnitude(&self, query: &[f64]) -> f64 {
        let sim = cosine_similarity(query, &self.embedding).clamp(0.0, 1.0);
        sim * self.source_trust.clamp(0.0, 1.0)
            * self.recency.clamp(0.0, 1.0)
            * self.graph_proximity.clamp(0.0, 1.0)
    }

    /// Phase: `π · contradiction`, plus a small novelty perturbation when phase
    /// kickback is enabled.
    fn phase(&self, phase_kickback: bool) -> f64 {
        let mut phase = std::f64::consts::PI * self.contradiction.clamp(0.0, 1.0);
        if phase_kickback {
            // A bounded nudge (<= ~0.1 rad) so novel-but-coherent evidence does
            // not perfectly overlap, without flipping its sign.
            phase += 0.1 * self.novelty.clamp(0.0, 1.0);
        }
        phase
    }
}

/// A single scored result from [`QuantumRagIndex::search`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoredCandidate {
    /// Candidate id.
    pub id: String,
    /// Candidate text.
    pub text: String,
    /// Post-interference selection probability (normalized over the field).
    pub score: f64,
    /// Final phase of the candidate's amplitude, in radians.
    pub phase: f64,
}

/// The outcome of an interference-reranked search.
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// Top-`k` candidates by post-interference probability (descending).
    pub selected: Vec<ScoredCandidate>,
    /// The collapse receipt for the (top-1) selection — auditable provenance.
    pub receipt: CollapseReceipt,
    /// The coherence-gate decision for the reranked field.
    pub gate: GateDecision,
    /// The top-`k` candidate ids under *plain cosine* ranking, for comparison.
    pub baseline_cosine_top_k: Vec<String>,
}

/// An interference-reranking retrieval index over [`RagCandidate`]s.
///
/// Build it with [`QuantumRagIndex::new`], tune with [`Self::interference_rounds`]
/// and [`Self::phase_kickback`], populate with [`Self::add`] / [`Self::add_many`],
/// then query with [`Self::search`].
#[derive(Debug, Clone)]
pub struct QuantumRagIndex {
    /// Embedding dimension the index expects (informational; mismatched lengths
    /// are tolerated by [`cosine_similarity`]).
    pub dim: usize,
    /// The indexed candidates.
    pub candidates: Vec<RagCandidate>,
    /// Number of field-level interference refinement rounds.
    pub interference_rounds: usize,
    /// Whether to apply a small novelty-driven phase perturbation.
    pub phase_kickback: bool,
}

impl QuantumRagIndex {
    /// Create an empty index for `dim`-dimensional embeddings, with `3`
    /// interference rounds and phase kickback enabled by default.
    pub fn new(dim: usize) -> Self {
        Self {
            dim,
            candidates: Vec::new(),
            interference_rounds: 3,
            phase_kickback: true,
        }
    }

    /// Set the number of interference refinement rounds (builder style).
    pub fn interference_rounds(mut self, n: usize) -> Self {
        self.interference_rounds = n;
        self
    }

    /// Enable / disable the novelty phase kickback (builder style).
    pub fn phase_kickback(mut self, on: bool) -> Self {
        self.phase_kickback = on;
        self
    }

    /// Add a single candidate.
    pub fn add(&mut self, candidate: RagCandidate) -> &mut Self {
        self.candidates.push(candidate);
        self
    }

    /// Add many candidates.
    pub fn add_many(&mut self, candidates: Vec<RagCandidate>) -> &mut Self {
        self.candidates.extend(candidates);
        self
    }

    /// Number of indexed candidates.
    pub fn len(&self) -> usize {
        self.candidates.len()
    }

    /// Whether the index is empty.
    pub fn is_empty(&self) -> bool {
        self.candidates.is_empty()
    }

    /// Build the initial possibility field for a query: amplitude = gated
    /// cosine, phase = `π·contradiction` (+ optional novelty kick), with one
    /// [`EvidenceReceipt`] attached per candidate.
    pub fn build_field(&self, query: &[f64]) -> PossibilityField<RagCandidate> {
        let possibilities = self
            .candidates
            .iter()
            .map(|c| {
                let magnitude = c.base_magnitude(query);
                let phase = c.phase(self.phase_kickback);
                let evidence =
                    EvidenceReceipt::new(c.id.clone(), c.source_trust, c.text.as_bytes());
                Possibility::new(c.id.clone(), c.clone(), magnitude, phase)
                    .with_evidence(vec![evidence])
            })
            .collect();
        PossibilityField::new(possibilities)
    }

    /// Run one interference round in place on a field.
    ///
    /// Each candidate's complex amplitude is projected onto the field's net
    /// (resultant) complex amplitude. The projection (`cos(Δphase)` relative to
    /// the resultant) is in `[-1, 1]`: candidates aligned with the resultant are
    /// amplified, opposed candidates decay. The update is
    /// `magnitude ← magnitude · (1 + alignment)`, clamped at zero, which keeps
    /// magnitudes non-negative and the whole step deterministic.
    fn interfere_once(field: &mut PossibilityField<RagCandidate>) {
        // Net (resultant) complex amplitude of the field.
        let mut net = Complex::ZERO;
        for c in &field.candidates {
            net += Complex::from_polar(c.amplitude, c.phase);
        }
        let net_norm = net.norm();
        if net_norm < 1e-15 {
            return;
        }
        let net_dir = net * (1.0 / net_norm);

        for c in &mut field.candidates {
            if c.amplitude < 1e-15 {
                continue;
            }
            let a = Complex::from_polar(c.amplitude, c.phase);
            // Alignment = projection of the unit candidate amplitude onto the
            // unit resultant = cos(phase difference) in [-1, 1].
            let alignment = (a.re * net_dir.re + a.im * net_dir.im) / c.amplitude;
            let factor = (1.0 + alignment).max(0.0);
            c.amplitude *= factor;
        }
    }

    /// Refine a field by running `interference_rounds` interference steps,
    /// renormalizing between rounds to keep magnitudes bounded.
    fn run_interference(&self, field: &mut PossibilityField<RagCandidate>) {
        for _ in 0..self.interference_rounds {
            Self::interfere_once(field);
            field.normalize();
        }
    }

    /// Interference-reranked search.
    ///
    /// Steps:
    /// 1. build the possibility field (amplitude/phase mapping),
    /// 2. run the interference rounds (constructive amplification / destructive
    ///    cancellation),
    /// 3. evaluate the [`CoherenceGate`] on the reranked field,
    /// 4. take the top-`k` by post-interference probability,
    /// 5. collapse with `seed` to emit the auditable [`CollapseReceipt`]
    ///    (`selected_id` is the interference top-1).
    ///
    /// Returns an error if the index is empty.
    pub fn search(&self, query: &[f64], k: usize, seed: u64) -> anyhow::Result<SearchResult> {
        if self.candidates.is_empty() {
            anyhow::bail!("cannot search an empty index");
        }

        // Baseline: plain cosine ranking (ids only). Ties broken by id for
        // determinism.
        let mut baseline: Vec<(String, f64)> = self
            .candidates
            .iter()
            .map(|c| (c.id.clone(), cosine_similarity(query, &c.embedding)))
            .collect();
        baseline.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.cmp(&b.0))
        });
        let baseline_cosine_top_k: Vec<String> =
            baseline.into_iter().take(k).map(|(id, _)| id).collect();

        // Interference reranking.
        let mut field = self.build_field(query);
        self.run_interference(&mut field);

        let coherence_gate = CoherenceGate::with_defaults();
        let gate = coherence_gate.evaluate(&field);

        // Rank candidates by post-interference probability. Ties broken by
        // lowest index so the top of this ranking matches the receipt's argmax
        // selection (see `collapse_argmax`).
        let probs = field.probabilities();
        let mut order: Vec<usize> = (0..field.candidates.len()).collect();
        order.sort_by(|&i, &j| {
            probs[j]
                .partial_cmp(&probs[i])
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| i.cmp(&j))
        });

        let scored: Vec<ScoredCandidate> = order
            .iter()
            .take(k)
            .map(|&i| {
                let c = &field.candidates[i];
                ScoredCandidate {
                    id: c.id.clone(),
                    text: c.payload.text.clone(),
                    score: probs[i],
                    phase: c.phase,
                }
            })
            .collect();

        // Collapse the reranked field to its argmax (the interference top-1) for
        // an auditable receipt. Deterministic, and the receipt's metrics
        // (coherence / entropy / gate / field hash) describe the field in its
        // natural order — so a replayer who reconstructs the same field via
        // `build_field` + interference reproduces the identical receipt.
        let (_selected, receipt) = field
            .collapse_argmax_with_gate(seed, &coherence_gate)
            .map_err(|e| anyhow::anyhow!("collapse failed: {e}"))?;

        Ok(SearchResult {
            selected: scored,
            receipt,
            gate,
            baseline_cosine_top_k,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cosine_basics() {
        assert!((cosine_similarity(&[1.0, 0.0], &[1.0, 0.0]) - 1.0).abs() < 1e-12);
        assert!(cosine_similarity(&[1.0, 0.0], &[0.0, 1.0]).abs() < 1e-12);
        assert!((cosine_similarity(&[1.0, 0.0], &[-1.0, 0.0]) + 1.0).abs() < 1e-12);
        // empty / zero norm
        assert_eq!(cosine_similarity(&[], &[1.0]), 0.0);
        assert_eq!(cosine_similarity(&[0.0, 0.0], &[1.0, 1.0]), 0.0);
    }

    #[test]
    fn contradiction_drives_phase_to_pi() {
        let c = RagCandidate::new("x", "t", vec![1.0, 0.0]).with_contradiction(1.0);
        // No kickback so phase is exactly pi.
        assert!((c.phase(false) - std::f64::consts::PI).abs() < 1e-12);
        let clean = RagCandidate::new("y", "t", vec![1.0, 0.0]);
        assert!(clean.phase(false).abs() < 1e-12);
    }

    #[test]
    fn empty_index_errors() {
        let index = QuantumRagIndex::new(2);
        assert!(index.search(&[1.0, 0.0], 3, 1).is_err());
    }
}
