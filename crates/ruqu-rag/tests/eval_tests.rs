//! Credible reranking benchmark — recommendation #2 of the ruqu SOTA report
//! (`docs/research/sota-landscape.md`).
//!
//! The old story — "+10% faithfulness over cosine" — is not a meaningful claim,
//! because **cosine is the universally-beaten weak floor**. This benchmark
//! replaces it with:
//!
//! * standard IR metrics (nDCG@10, Recall@10, MRR) from [`ruqu_rag::eval`],
//! * a **strong** opponent (a *simulated* supervised cross-encoder / ColBERTv2,
//!   [`ruqu_rag::baseline::supervised_ranking`]) in addition to the weak cosine
//!   floor, and
//! * a RAG-grounding axis (context-precision@10 / contradiction-rate@10).
//!
//! It asserts only **honest, defensible** claims:
//!
//! (a) interference beats *cosine* on nDCG@10 AND context-precision@10
//!     (it should — that is the easy, weak-floor comparison); and
//! (b) interference's advantage over the *strong* baseline shows up
//!     specifically on **contradiction suppression** (context-precision@10 /
//!     contradiction-rate@10), even though the strong supervised baseline —
//!     which is essentially a noisy relevance oracle — wins raw nDCG@10. We do
//!     NOT claim interference beats the strong baseline on nDCG; we assert what
//!     is actually true and document it.
//!
//! Everything is deterministic.

use std::collections::HashSet;

use ruqu_rag::{
    baseline, context_precision_at_k, contradiction_rate_at_k, mrr, ndcg_at_k, recall_at_k,
    QuantumRagIndex, RagCandidate, Relevance,
};

const DIM: usize = 8;
const K: usize = 10;

/// A tiny deterministic LCG so the test needs no external RNG features.
struct Lcg(u64);
impl Lcg {
    fn new(seed: u64) -> Self {
        Lcg(seed)
    }
    /// Uniform in [-1, 1).
    fn next_unit(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let x = (self.0 >> 11) as f64 / (1u64 << 53) as f64;
        2.0 * x - 1.0
    }
}

fn query() -> Vec<f64> {
    let mut q = vec![0.0; DIM];
    q[0] = 1.0;
    q
}

/// Embedding near the query direction (high cosine), with deterministic jitter.
fn near_query(jitter: f64, rng: &mut Lcg) -> Vec<f64> {
    let mut e = vec![0.0; DIM];
    e[0] = 1.0;
    for v in e.iter_mut().skip(1) {
        *v = jitter * rng.next_unit();
    }
    e
}

/// Embedding orthogonal to the query (low cosine) — a distractor.
fn distractor(rng: &mut Lcg) -> Vec<f64> {
    let mut e = vec![0.0; DIM];
    for v in e.iter_mut().skip(1) {
        *v = rng.next_unit();
    }
    if e.iter().all(|&x| x.abs() < 1e-9) {
        e[1] = 1.0;
    }
    e
}

/// The synthetic corpus: 200 candidates with graded relevance + a contradiction
/// set, mirroring the report's "relevant / contradictory-but-high-cosine /
/// distractor" decomposition.
struct Dataset {
    candidates: Vec<RagCandidate>,
    relevance: Relevance,
    contradicted: HashSet<String>,
}

/// Build the dataset deterministically.
///
/// - 20 RELEVANT: high cosine, graded relevance (3 = top, 2 = good), no
///   contradiction.
/// - 20 CONTRADICTORY: ALSO high cosine (so cosine ranks them high), and *also
///   topically relevant* (grade 2) — a generic supervised reranker has no
///   reason to drop them — but they contradict the grounded answer, so they are
///   in the contradiction set and should be suppressed.
/// - 160 DISTRACTORS: low cosine, grade 0.
fn build() -> Dataset {
    let mut rng = Lcg::new(0xC0FFEE);
    let mut candidates = Vec::with_capacity(200);
    let mut relevance = Relevance::new();
    let mut contradicted = HashSet::new();

    for i in 0..20 {
        let id = format!("rel_{i:03}");
        let e = near_query(0.10, &mut rng);
        candidates.push(RagCandidate::new(&id, format!("relevant chunk {i}"), e));
        // Graded relevance: first few are the most relevant.
        let grade = if i < 5 { 3.0 } else { 2.0 };
        relevance.set(&id, grade);
    }

    for i in 0..20 {
        let id = format!("con_{i:03}");
        let e = near_query(0.10, &mut rng);
        // Topically relevant (grade 2) — a strong supervised reranker, trained
        // for topical relevance, will happily rank these high. But they
        // contradict the grounded answer, so they belong in the contradiction
        // set and the interference phase flips them to ~pi.
        candidates.push(
            RagCandidate::new(&id, format!("contradictory chunk {i}"), e)
                .with_contradiction(1.0)
                .with_source_trust(0.85),
        );
        relevance.set(&id, 2.0);
        contradicted.insert(id);
    }

    for i in 0..160 {
        let id = format!("dis_{i:03}");
        let e = distractor(&mut rng);
        candidates.push(RagCandidate::new(&id, format!("distractor chunk {i}"), e));
        // grade 0 (implicitly not relevant).
        let _ = i;
    }

    Dataset {
        candidates,
        relevance,
        contradicted,
    }
}

/// The interference ranking as a full id list (rank all candidates).
fn interference_ranking(ds: &Dataset, q: &[f64]) -> Vec<String> {
    let mut index = QuantumRagIndex::new(DIM).interference_rounds(4);
    index.add_many(ds.candidates.clone());
    // Ask for the whole field so the ranking is complete for the metrics.
    let result = index.search(q, ds.candidates.len(), 7).expect("search");
    result.selected.into_iter().map(|s| s.id).collect()
}

/// A row of the metric table.
struct Row {
    name: &'static str,
    ndcg: f64,
    recall: f64,
    mrr: f64,
    ctx_prec: f64,
    contra_rate: f64,
}

fn measure(name: &'static str, ranking: &[String], ds: &Dataset) -> Row {
    Row {
        name,
        ndcg: ndcg_at_k(ranking, &ds.relevance, K),
        recall: recall_at_k(ranking, &ds.relevance, K),
        mrr: mrr(ranking, &ds.relevance),
        ctx_prec: context_precision_at_k(ranking, &ds.relevance, &ds.contradicted, K),
        contra_rate: contradiction_rate_at_k(ranking, &ds.contradicted, K),
    }
}

fn print_table(rows: &[Row]) {
    println!(
        "\n{:<14} {:>9} {:>10} {:>7} {:>13} {:>13}",
        "reranker", "nDCG@10", "Recall@10", "MRR", "ctx-prec@10", "contra@10"
    );
    println!("{}", "-".repeat(70));
    for r in rows {
        println!(
            "{:<14} {:>9.3} {:>10.3} {:>7.3} {:>13.3} {:>13.3}",
            r.name, r.ndcg, r.recall, r.mrr, r.ctx_prec, r.contra_rate
        );
    }
    println!();
}

#[test]
fn credible_reranking_benchmark() {
    let ds = build();
    let q = query();

    // Weak floor.
    let cosine = baseline::cosine_ranking(&q, &ds.candidates);
    // Strong opponent: simulated supervised cross-encoder (noisy relevance
    // oracle). NOT a real cross-encoder — see baseline::supervised_ranking docs
    // and docs/research/sota-landscape.md for the real external validation.
    let strong = baseline::supervised_ranking(&ds.candidates, &ds.relevance, 0.4, 1234);
    // Interference reranker.
    let interference = interference_ranking(&ds, &q);

    let rows = vec![
        measure("cosine(weak)", &cosine, &ds),
        measure("supervised(strong)", &strong, &ds),
        measure("interference", &interference, &ds),
    ];
    print_table(&rows);

    let cos = &rows[0];
    let strong_r = &rows[1];
    let intf = &rows[2];

    // (a) Interference beats the WEAK cosine floor on nDCG@10 AND
    //     context-precision@10. This is the easy, defensible comparison.
    assert!(
        intf.ndcg > cos.ndcg,
        "interference nDCG@10 ({:.3}) must beat cosine ({:.3})",
        intf.ndcg,
        cos.ndcg
    );
    assert!(
        intf.ctx_prec > cos.ctx_prec,
        "interference ctx-prec@10 ({:.3}) must beat cosine ({:.3})",
        intf.ctx_prec,
        cos.ctx_prec
    );

    // (b) The HONEST claim vs the STRONG baseline: interference wins on
    //     CONTRADICTION SUPPRESSION (higher context-precision, lower
    //     contradiction-rate), even though the strong supervised baseline — a
    //     noisy relevance oracle — wins raw nDCG@10. We assert exactly that,
    //     and do not fabricate an nDCG win.
    assert!(
        intf.ctx_prec > strong_r.ctx_prec,
        "interference ctx-prec@10 ({:.3}) must beat the strong baseline ({:.3}) \
         on contradiction-aware grounding",
        intf.ctx_prec,
        strong_r.ctx_prec
    );
    assert!(
        intf.contra_rate < strong_r.contra_rate,
        "interference contradiction-rate@10 ({:.3}) must be lower than the strong \
         baseline ({:.3})",
        intf.contra_rate,
        strong_r.contra_rate
    );

    // The strong baseline is genuinely strong: it should win (or tie) raw
    // nDCG@10. Documenting this keeps us honest — interference is NOT a
    // universal winner; its edge is grounding/contradiction suppression.
    assert!(
        strong_r.ndcg >= intf.ndcg - 1e-9,
        "the simulated strong baseline should not be beaten by interference on raw \
         nDCG@10 (strong {:.3} vs interference {:.3}); if it is, the honesty note \
         in the README must be updated",
        strong_r.ndcg,
        intf.ndcg
    );

    // Interference should let essentially no contradicted chunks into the
    // top-10 (the whole point of the phase-pi suppression).
    assert!(
        intf.contra_rate <= 0.1,
        "interference top-10 should be ~free of contradicted chunks, got rate {:.3}",
        intf.contra_rate
    );
}

#[test]
fn metrics_are_deterministic() {
    let ds = build();
    let q = query();
    let a = interference_ranking(&ds, &q);
    let b = interference_ranking(&ds, &q);
    assert_eq!(a, b, "interference ranking must be deterministic");

    let s1 = baseline::supervised_ranking(&ds.candidates, &ds.relevance, 0.4, 1234);
    let s2 = baseline::supervised_ranking(&ds.candidates, &ds.relevance, 0.4, 1234);
    assert_eq!(s1, s2, "simulated strong baseline must be deterministic");
}
