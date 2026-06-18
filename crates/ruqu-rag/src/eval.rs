//! # Credible IR evaluation for interference reranking
//!
//! This module exists because "+10% faithfulness over cosine" is **not a
//! meaningful claim**: cosine is the universally-beaten *weak floor* of the
//! reranking literature. The ruqu SOTA report
//! (`docs/research/sota-landscape.md`, recommendation #2) is blunt about this —
//! the real bar is a *strong supervised reranker* (cross-encoder / ColBERTv2)
//! measured with standard IR metrics on real corpora, plus a faithfulness /
//! context-precision axis on conflicting-evidence benchmarks.
//!
//! So this module provides:
//!
//! * standard **ranking metrics** over a ranked list of ids given graded /
//!   binary relevance labels: [`dcg_at_k`], [`ndcg_at_k`], [`recall_at_k`],
//!   [`precision_at_k`], [`mrr`], [`average_precision`] (MAP), and
//! * a RAG-grounding axis: [`context_precision_at_k`] — the fraction of the
//!   top-`k` that are *relevant **and** not contradicted* (a faithfulness
//!   proxy);
//! * a [`baseline`] submodule with the weak cosine floor **and** a *simulated*
//!   strong supervised reranker to compare against honestly.
//!
//! All metric functions are pure, take ids by reference, and degrade
//! gracefully on empty / short rankings and `k > len`.
//!
//! ## What this is NOT
//!
//! These are **offline metric implementations and a synthetic comparison
//! harness**, not external validation. The report names the proper external
//! baselines and datasets (see [`baseline::supervised_ranking`] and the crate
//! README): a real cross-encoder (BGE-reranker-v2-m3) / ColBERTv2 on
//! **BEIR / MS MARCO**, RAGAS faithfulness + context-precision, and
//! contradiction-suppression benchmarks (**RGB-counterfactual, FaithEval,
//! RAMDocs**). Those require model weights and corpora that cannot run here.

use std::collections::{HashMap, HashSet};

/// A graded-relevance oracle: maps an id to a non-negative relevance grade.
///
/// A grade of `0` (or an absent id) means *not relevant*. Larger grades mean
/// *more* relevant; binary relevance is the special case of grades in
/// `{0, 1}`. Used by the graded metrics ([`dcg_at_k`] / [`ndcg_at_k`]) and, via
/// [`Relevance::is_relevant`], by the binary metrics.
#[derive(Debug, Clone, Default)]
pub struct Relevance {
    grades: HashMap<String, f64>,
}

impl Relevance {
    /// An empty oracle (everything is non-relevant).
    pub fn new() -> Self {
        Self {
            grades: HashMap::new(),
        }
    }

    /// Build from `(id, grade)` pairs. Negative grades are clamped to `0`.
    pub fn from_pairs<I, S>(pairs: I) -> Self
    where
        I: IntoIterator<Item = (S, f64)>,
        S: Into<String>,
    {
        let mut r = Relevance::new();
        for (id, g) in pairs {
            r.set(id, g);
        }
        r
    }

    /// Set the grade for an id (negative grades are clamped to `0`).
    pub fn set(&mut self, id: impl Into<String>, grade: f64) -> &mut Self {
        self.grades.insert(id.into(), grade.max(0.0));
        self
    }

    /// The graded relevance of an id (`0.0` if unknown).
    pub fn grade(&self, id: &str) -> f64 {
        self.grades.get(id).copied().unwrap_or(0.0)
    }

    /// Binary relevance: `true` iff the grade is `> 0`.
    pub fn is_relevant(&self, id: &str) -> bool {
        self.grade(id) > 0.0
    }

    /// Total number of relevant items in the oracle (grade `> 0`). This is the
    /// denominator for [`recall_at_k`].
    pub fn total_relevant(&self) -> usize {
        self.grades.values().filter(|&&g| g > 0.0).count()
    }
}

/// Effective cutoff: `k`, but never more than the ranking length, and `0` stays
/// `0`. Centralises the "k > len" handling for every metric.
#[inline]
fn effective_k(ranking_len: usize, k: usize) -> usize {
    k.min(ranking_len)
}

/// Discounted Cumulative Gain at `k` (graded relevance, log2 discount).
///
/// `DCG@k = Σ_{i=1..k} grade(rankingᵢ) / log2(i + 1)`.
///
/// Returns `0.0` for an empty ranking or `k == 0`. `k` larger than the ranking
/// is treated as the full ranking length.
pub fn dcg_at_k(ranking: &[String], relevance: &Relevance, k: usize) -> f64 {
    let kk = effective_k(ranking.len(), k);
    let mut dcg = 0.0;
    for (i, id) in ranking.iter().take(kk).enumerate() {
        let grade = relevance.grade(id);
        if grade != 0.0 {
            // rank position is i + 1, discount is log2((i + 1) + 1).
            dcg += grade / ((i as f64) + 2.0).log2();
        }
    }
    dcg
}

/// Normalized DCG at `k`: [`dcg_at_k`] divided by the ideal DCG (the DCG of the
/// best possible ordering of the *available* graded items).
///
/// The ideal ranking is built from the relevance oracle's grades, so nDCG is in
/// `[0, 1]` regardless of how many relevant items the supplied ranking missed.
/// Returns `0.0` when there is no attainable gain (ideal DCG is `0`), which
/// keeps the metric well-defined for queries with no relevant items.
pub fn ndcg_at_k(ranking: &[String], relevance: &Relevance, k: usize) -> f64 {
    let dcg = dcg_at_k(ranking, relevance, k);

    // Ideal DCG: sort *all* known positive grades descending and take the top-k.
    let mut ideal: Vec<f64> = relevance
        .grades
        .values()
        .copied()
        .filter(|&g| g > 0.0)
        .collect();
    ideal.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));

    let kk = k.min(ideal.len());
    let mut idcg = 0.0;
    for (i, &grade) in ideal.iter().take(kk).enumerate() {
        idcg += grade / ((i as f64) + 2.0).log2();
    }

    if idcg <= 0.0 {
        0.0
    } else {
        dcg / idcg
    }
}

/// Recall at `k`: fraction of *all* relevant items (binary) that appear in the
/// top-`k`.
///
/// Denominator is [`Relevance::total_relevant`]. Returns `0.0` when there are no
/// relevant items. Duplicate ids in the ranking are counted once.
pub fn recall_at_k(ranking: &[String], relevance: &Relevance, k: usize) -> f64 {
    let total = relevance.total_relevant();
    if total == 0 {
        return 0.0;
    }
    let kk = effective_k(ranking.len(), k);
    let mut seen: HashSet<&str> = HashSet::new();
    let mut hits = 0usize;
    for id in ranking.iter().take(kk) {
        if relevance.is_relevant(id) && seen.insert(id.as_str()) {
            hits += 1;
        }
    }
    hits as f64 / total as f64
}

/// Precision at `k`: fraction of the top-`k` positions that are relevant
/// (binary).
///
/// The denominator is the effective cutoff (`min(k, len)`), so a short ranking
/// is not unfairly penalised by empty slots. Returns `0.0` for an empty ranking
/// or `k == 0`.
pub fn precision_at_k(ranking: &[String], relevance: &Relevance, k: usize) -> f64 {
    let kk = effective_k(ranking.len(), k);
    if kk == 0 {
        return 0.0;
    }
    let hits = ranking
        .iter()
        .take(kk)
        .filter(|id| relevance.is_relevant(id))
        .count();
    hits as f64 / kk as f64
}

/// Mean Reciprocal Rank for a single ranking: `1 / rank` of the first relevant
/// item (binary), or `0.0` if none is relevant.
///
/// ("Mean" applies when averaged over queries; for one ranking this is the
/// reciprocal rank.)
pub fn mrr(ranking: &[String], relevance: &Relevance) -> f64 {
    for (i, id) in ranking.iter().enumerate() {
        if relevance.is_relevant(id) {
            return 1.0 / ((i + 1) as f64);
        }
    }
    0.0
}

/// Average Precision (the per-query term of MAP) over the full ranking.
///
/// `AP = (Σ_k precision@k · rel_k) / (#relevant)`, where the sum runs over ranks
/// `k` whose item is relevant and `rel_k ∈ {0,1}`. Returns `0.0` when there are
/// no relevant items. Duplicate ids are credited only on first appearance.
pub fn average_precision(ranking: &[String], relevance: &Relevance) -> f64 {
    let total = relevance.total_relevant();
    if total == 0 {
        return 0.0;
    }
    let mut seen: HashSet<&str> = HashSet::new();
    let mut hits = 0usize;
    let mut sum = 0.0;
    for (i, id) in ranking.iter().enumerate() {
        if relevance.is_relevant(id) && seen.insert(id.as_str()) {
            hits += 1;
            sum += hits as f64 / ((i + 1) as f64);
        }
    }
    sum / total as f64
}

/// Context-precision @ `k` — the RAG-grounding / faithfulness proxy.
///
/// Fraction of the top-`k` retrieved chunks that are **relevant AND not
/// contradicted**. This is the axis where structural rerankers should earn
/// their keep: a chunk that is semantically on-topic *but contradicts the
/// grounded answer* (high cosine, wrong content) pollutes the context window and
/// drives hallucination. It is excluded here even if its relevance grade is
/// positive.
///
/// `contradicted` is the set of ids flagged as contradicting the grounded
/// answer (e.g. by an NLI check or, here, the synthetic contradiction set). The
/// denominator is the effective cutoff (`min(k, len)`); returns `0.0` for an
/// empty ranking or `k == 0`.
///
/// This mirrors RAGAS "context precision" / "faithfulness" in spirit; the
/// report's real external validation is RAGAS on RGB / FaithEval / RAMDocs.
pub fn context_precision_at_k(
    ranking: &[String],
    relevance: &Relevance,
    contradicted: &HashSet<String>,
    k: usize,
) -> f64 {
    let kk = effective_k(ranking.len(), k);
    if kk == 0 {
        return 0.0;
    }
    let grounded = ranking
        .iter()
        .take(kk)
        .filter(|id| relevance.is_relevant(id) && !contradicted.contains(id.as_str()))
        .count();
    grounded as f64 / kk as f64
}

/// Fraction of the top-`k` that are **contradicted** (lower is better).
///
/// This is the direct contradiction-suppression metric: it isolates how much
/// contradictory-but-plausible evidence each reranker lets into the context
/// window, independent of overall relevance. The report identifies this as the
/// axis where interference reranking is differentiated.
pub fn contradiction_rate_at_k(
    ranking: &[String],
    contradicted: &HashSet<String>,
    k: usize,
) -> f64 {
    let kk = effective_k(ranking.len(), k);
    if kk == 0 {
        return 0.0;
    }
    let bad = ranking
        .iter()
        .take(kk)
        .filter(|id| contradicted.contains(id.as_str()))
        .count();
    bad as f64 / kk as f64
}

/// Strong and weak baseline rerankers to compare interference against.
///
/// The SOTA report (`docs/research/sota-landscape.md`) is explicit: cosine is
/// the **weak floor**, and a credible claim must compare against a *strong
/// supervised reranker* — a cross-encoder (e.g. BGE-reranker-v2-m3, ~0.66 avg
/// nDCG@10 on BEIR) or **ColBERTv2** (~39.7% MRR@10 on MS MARCO, ~50 nDCG@10 on
/// BEIR), with RAGAS faithfulness on RGB / FaithEval / RAMDocs.
///
/// We cannot run a real cross-encoder in this crate (no model weights, no
/// network). [`supervised_ranking`] therefore **simulates** one: it ranks by the
/// (noisy) ground-truth graded relevance, which is the best a perfectly trained
/// supervised reranker could do, degraded by a controllable noise term. This
/// gives a *much stronger* and *fairer* comparison point than cosine, while
/// being honest that it is a stand-in, not a measured model.
pub mod baseline {
    use super::Relevance;
    use crate::{cosine_similarity, RagCandidate};

    /// **Weak floor.** Rank candidate ids by plain cosine similarity to the
    /// query, descending. Ties broken by id for determinism. This is the
    /// baseline the report says is meaningless to "beat" on its own.
    pub fn cosine_ranking(query: &[f64], candidates: &[RagCandidate]) -> Vec<String> {
        let mut scored: Vec<(String, f64)> = candidates
            .iter()
            .map(|c| (c.id.clone(), cosine_similarity(query, &c.embedding)))
            .collect();
        scored.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.cmp(&b.0))
        });
        scored.into_iter().map(|(id, _)| id).collect()
    }

    /// **Simulated strong supervised reranker.**
    ///
    /// THIS IS NOT A REAL CROSS-ENCODER. It is a deterministic stand-in for a
    /// trained supervised reranker (cross-encoder / ColBERTv2), used to give the
    /// interference reranker a *strong*, fair opponent for relative comparison.
    /// A perfectly trained supervised reranker approximates the ground-truth
    /// graded relevance; this function ranks by exactly that, perturbed by a
    /// bounded, seeded noise term (`noise`, the imperfection of a real model).
    ///
    /// Crucially — and this is the point of recommendation #2 — a generic
    /// supervised reranker is trained for *topical relevance*, not *answer
    /// grounding*: a contradicted chunk is still highly topically relevant, so a
    /// strong reranker will happily rank it near the top. This simulation
    /// therefore ranks contradicted chunks by their relevance grade like any
    /// other chunk; it does **not** get the contradiction-suppression for free.
    /// That is exactly where interference reranking is expected to differentiate
    /// (context-precision / contradiction-rate), even when the strong baseline
    /// wins raw nDCG.
    ///
    /// Real external validation must use a *measured* cross-encoder / ColBERTv2
    /// on BEIR / MS MARCO with RAGAS faithfulness on RGB / FaithEval / RAMDocs
    /// (see `docs/research/sota-landscape.md`). `noise = 0.0` gives the oracle
    /// upper bound (the ideal ranking).
    pub fn supervised_ranking(
        candidates: &[RagCandidate],
        relevance: &Relevance,
        noise: f64,
        seed: u64,
    ) -> Vec<String> {
        // Deterministic per-id noise via a SplitMix64 hash of (seed, id). This
        // makes the simulated reranker reproducible and order-independent.
        let mut scored: Vec<(String, f64)> = candidates
            .iter()
            .map(|c| {
                let base = relevance.grade(&c.id);
                let jitter = noise * symmetric_unit(seed, &c.id);
                (c.id.clone(), base + jitter)
            })
            .collect();
        scored.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.cmp(&b.0))
        });
        scored.into_iter().map(|(id, _)| id).collect()
    }

    /// Deterministic value in `[-1, 1]` from `(seed, id)` via SplitMix64.
    fn symmetric_unit(seed: u64, id: &str) -> f64 {
        let mut x = seed;
        for b in id.as_bytes() {
            x = x.wrapping_add(*b as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        }
        // SplitMix64 finalizer.
        x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        x ^= x >> 31;
        let u = (x >> 11) as f64 / (1u64 << 53) as f64; // [0, 1)
        2.0 * u - 1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    fn set(v: &[&str]) -> HashSet<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn relevance_basics() {
        let r = Relevance::from_pairs([("a", 3.0), ("b", 1.0), ("c", -2.0), ("d", 0.0)]);
        assert_eq!(r.grade("a"), 3.0);
        assert_eq!(r.grade("c"), 0.0); // clamped
        assert_eq!(r.grade("missing"), 0.0);
        assert!(r.is_relevant("a"));
        assert!(!r.is_relevant("c"));
        assert!(!r.is_relevant("d"));
        // a and b are relevant; c clamped to 0, d is 0.
        assert_eq!(r.total_relevant(), 2);
    }

    #[test]
    fn dcg_and_ndcg_perfect_and_empty() {
        let r = Relevance::from_pairs([("a", 3.0), ("b", 2.0), ("c", 1.0)]);
        // Ideal order a,b,c.
        let perfect = ids(&["a", "b", "c"]);
        // nDCG of the ideal ranking is 1.0.
        assert!((ndcg_at_k(&perfect, &r, 3) - 1.0).abs() < 1e-12);

        // A worse order scores < 1.
        let worse = ids(&["c", "b", "a"]);
        assert!(ndcg_at_k(&worse, &r, 3) < 1.0);

        // Empty ranking / k = 0 / no relevant items all degrade to 0.
        assert_eq!(dcg_at_k(&[], &r, 5), 0.0);
        assert_eq!(ndcg_at_k(&perfect, &r, 0), 0.0);
        let empty_rel = Relevance::new();
        assert_eq!(ndcg_at_k(&perfect, &empty_rel, 3), 0.0);
    }

    #[test]
    fn dcg_known_value() {
        // grades 3, 2: DCG@2 = 3/log2(2) + 2/log2(3) = 3 + 2/1.58496 = 4.2619.
        let r = Relevance::from_pairs([("a", 3.0), ("b", 2.0)]);
        let ranking = ids(&["a", "b"]);
        let dcg = dcg_at_k(&ranking, &r, 2);
        assert!((dcg - (3.0 + 2.0 / 3.0_f64.log2())).abs() < 1e-9);
    }

    #[test]
    fn ndcg_k_greater_than_len_is_graceful() {
        let r = Relevance::from_pairs([("a", 1.0), ("b", 1.0)]);
        let ranking = ids(&["a", "b"]);
        // k = 100 > 2: treated as full length, perfect order -> 1.0.
        assert!((ndcg_at_k(&ranking, &r, 100) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn recall_precision() {
        let r = Relevance::from_pairs([("a", 1.0), ("b", 1.0), ("c", 1.0), ("d", 1.0)]);
        let ranking = ids(&["a", "x", "b", "y"]);
        // 2 of 4 relevant in top-4.
        assert!((recall_at_k(&ranking, &r, 4) - 0.5).abs() < 1e-12);
        // top-2: a relevant, x not -> precision 0.5.
        assert!((precision_at_k(&ranking, &r, 2) - 0.5).abs() < 1e-12);
        // no relevant items -> recall 0.
        assert_eq!(recall_at_k(&ranking, &Relevance::new(), 4), 0.0);
        // empty ranking / k=0.
        assert_eq!(precision_at_k(&[], &r, 4), 0.0);
        assert_eq!(precision_at_k(&ranking, &r, 0), 0.0);
    }

    #[test]
    fn recall_dedups() {
        let r = Relevance::from_pairs([("a", 1.0), ("b", 1.0)]);
        // duplicate a should not double-count.
        let ranking = ids(&["a", "a", "b"]);
        assert!((recall_at_k(&ranking, &r, 3) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn mrr_and_ap() {
        let r = Relevance::from_pairs([("a", 1.0), ("b", 1.0)]);
        // first relevant at rank 2 -> 0.5.
        let ranking = ids(&["x", "a", "y", "b"]);
        assert!((mrr(&ranking, &r) - 0.5).abs() < 1e-12);
        // AP = (1/2 + 2/4) / 2 = 0.5.
        assert!((average_precision(&ranking, &r) - 0.5).abs() < 1e-12);
        // no relevant -> both 0.
        assert_eq!(mrr(&ranking, &Relevance::new()), 0.0);
        assert_eq!(average_precision(&ranking, &Relevance::new()), 0.0);
        // perfect ranking AP = 1.
        let perfect = ids(&["a", "b"]);
        assert!((average_precision(&perfect, &r) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn context_precision_excludes_contradicted() {
        let r = Relevance::from_pairs([("a", 1.0), ("b", 1.0), ("c", 1.0)]);
        let contradicted = set(&["b"]);
        let ranking = ids(&["a", "b", "c"]);
        // b is relevant but contradicted -> excluded. 2/3 grounded.
        assert!((context_precision_at_k(&ranking, &r, &contradicted, 3) - 2.0 / 3.0).abs() < 1e-12);
        // contradiction rate: 1/3.
        assert!((contradiction_rate_at_k(&ranking, &contradicted, 3) - 1.0 / 3.0).abs() < 1e-12);
        // empty / k=0.
        assert_eq!(context_precision_at_k(&[], &r, &contradicted, 3), 0.0);
        assert_eq!(contradiction_rate_at_k(&ranking, &contradicted, 0), 0.0);
    }

    #[test]
    fn baseline_supervised_oracle_is_perfect_with_zero_noise() {
        use crate::RagCandidate;
        let cands = vec![
            RagCandidate::new("a", "", vec![1.0]),
            RagCandidate::new("b", "", vec![1.0]),
            RagCandidate::new("c", "", vec![1.0]),
        ];
        let r = Relevance::from_pairs([("a", 1.0), ("b", 2.0), ("c", 3.0)]);
        let ranking = baseline::supervised_ranking(&cands, &r, 0.0, 42);
        // With no noise, ranks by grade descending: c, b, a.
        assert_eq!(ranking, ids(&["c", "b", "a"]));
        // Deterministic across calls.
        let again = baseline::supervised_ranking(&cands, &r, 0.3, 42);
        let again2 = baseline::supervised_ranking(&cands, &r, 0.3, 42);
        assert_eq!(again, again2);
    }

    #[test]
    fn cosine_ranking_orders_by_similarity() {
        use crate::RagCandidate;
        let q = vec![1.0, 0.0];
        let cands = vec![
            RagCandidate::new("far", "", vec![0.0, 1.0]),
            RagCandidate::new("near", "", vec![1.0, 0.0]),
            RagCandidate::new("mid", "", vec![1.0, 1.0]),
        ];
        let ranking = baseline::cosine_ranking(&q, &cands);
        assert_eq!(ranking[0], "near");
        assert_eq!(ranking.last().unwrap(), "far");
    }
}
