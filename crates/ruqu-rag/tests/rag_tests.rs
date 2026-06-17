//! ADR-258 §22 Test 1 — retrieval coherence (citation faithfulness).
//!
//! Plain cosine reranking cannot tell a *relevant* chunk from a *contradictory*
//! one when both are semantically close to the query — they have (near)
//! identical cosine. Interference reranking can: contradictory chunks carry a
//! phase near `π` and destructively cancel against the coherent bulk, so they
//! fall out of the top results.
//!
//! This test builds 200 candidates (20 relevant, 20 contradictory, 160
//! distractors) and asserts the interference reranker's top-10 citation
//! faithfulness beats plain cosine's by at least 10 percentage points. The data
//! is fully deterministic.

use ruqu_rag::{cosine_similarity, QuantumRagIndex, RagCandidate};

const DIM: usize = 8;

/// A tiny deterministic LCG so the test needs no external RNG crate features.
struct Lcg(u64);
impl Lcg {
    fn new(seed: u64) -> Self {
        Lcg(seed)
    }
    /// Uniform in [-1, 1).
    fn next_unit(&mut self) -> f64 {
        // Numerical Recipes LCG constants.
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let x = (self.0 >> 11) as f64 / (1u64 << 53) as f64; // [0,1)
        2.0 * x - 1.0
    }
}

/// The query points along the first axis.
fn query() -> Vec<f64> {
    let mut q = vec![0.0; DIM];
    q[0] = 1.0;
    q
}

/// An embedding near the query direction, with a small deterministic jitter in
/// the orthogonal dimensions so candidates are distinct but all highly similar.
fn near_query(jitter: f64, rng: &mut Lcg) -> Vec<f64> {
    let mut e = vec![0.0; DIM];
    e[0] = 1.0;
    for v in e.iter_mut().skip(1) {
        *v = jitter * rng.next_unit();
    }
    e
}

/// An embedding orthogonal to the query (zero first component), random in the
/// remaining dimensions — a distractor with low cosine to the query.
fn distractor(rng: &mut Lcg) -> Vec<f64> {
    let mut e = vec![0.0; DIM];
    for v in e.iter_mut().skip(1) {
        *v = rng.next_unit();
    }
    // Avoid the degenerate all-zero vector.
    if e.iter().all(|&x| x.abs() < 1e-9) {
        e[1] = 1.0;
    }
    e
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Kind {
    Relevant,
    Contradictory,
    Distractor,
}

fn build() -> (QuantumRagIndex, std::collections::HashMap<String, Kind>) {
    let mut rng = Lcg::new(0xC0FFEE);
    let mut index = QuantumRagIndex::new(DIM).interference_rounds(4);
    let mut kinds = std::collections::HashMap::new();

    // 20 RELEVANT: near the query, no contradiction, phase ~ 0.
    for i in 0..20 {
        let id = format!("rel_{i:03}");
        let e = near_query(0.10, &mut rng);
        index.add(RagCandidate::new(&id, format!("relevant chunk {i}"), e));
        kinds.insert(id, Kind::Relevant);
    }

    // 20 CONTRADICTORY: ALSO near the query (so plain cosine ranks them high),
    // but contradiction ~ 1.0 so phase ~ pi.
    for i in 0..20 {
        let id = format!("con_{i:03}");
        let e = near_query(0.10, &mut rng);
        // Contradicted sources are typically a little less trusted; cosine
        // ignores this, so they still rank high under plain cosine, but it lets
        // the coherent bulk dominate the interference resultant.
        index.add(
            RagCandidate::new(&id, format!("contradictory chunk {i}"), e)
                .with_contradiction(1.0)
                .with_source_trust(0.85),
        );
        kinds.insert(id, Kind::Contradictory);
    }

    // 160 DISTRACTORS: orthogonal/random, low similarity.
    for i in 0..160 {
        let id = format!("dis_{i:03}");
        let e = distractor(&mut rng);
        index.add(RagCandidate::new(&id, format!("distractor chunk {i}"), e));
        kinds.insert(id, Kind::Distractor);
    }

    (index, kinds)
}

/// Fraction of `ids` that are RELEVANT.
fn faithfulness(ids: &[String], kinds: &std::collections::HashMap<String, Kind>) -> f64 {
    if ids.is_empty() {
        return 0.0;
    }
    let rel = ids
        .iter()
        .filter(|id| kinds.get(*id) == Some(&Kind::Relevant))
        .count();
    rel as f64 / ids.len() as f64
}

#[test]
fn test1_retrieval_coherence_faithfulness() {
    let (index, kinds) = build();
    let q = query();

    // Interference reranker top-10 (its own baseline cosine list comes free).
    let result = index.search(&q, 10, 7).expect("search");
    let interference_top10: Vec<String> = result.selected.iter().map(|s| s.id.clone()).collect();
    let cosine_top10 = result.baseline_cosine_top_k.clone();

    let cosine_f = faithfulness(&cosine_top10, &kinds);
    let interference_f = faithfulness(&interference_top10, &kinds);

    println!("cosine top-10 faithfulness       = {cosine_f:.3}");
    println!("interference top-10 faithfulness = {interference_f:.3}");
    println!("gate decision                    = {}", result.gate.as_str());

    // Plain cosine cannot separate relevant from contradictory (both high sim),
    // so it should hover around 0.5. Interference suppresses the contradictory
    // ones and should reach ~1.0.
    assert!(
        interference_f >= cosine_f + 0.10,
        "interference faithfulness ({interference_f:.3}) must exceed cosine ({cosine_f:.3}) by >= 0.10"
    );

    // The contradictory chunks should be largely absent from the interference
    // top-10.
    let contradictory_in_top10 = interference_top10
        .iter()
        .filter(|id| kinds.get(*id) == Some(&Kind::Contradictory))
        .count();
    assert!(
        contradictory_in_top10 <= 1,
        "interference top-10 should be free of contradictory chunks, found {contradictory_in_top10}"
    );
}

#[test]
fn cosine_similarity_correctness() {
    assert!((cosine_similarity(&[1.0, 2.0, 3.0], &[1.0, 2.0, 3.0]) - 1.0).abs() < 1e-12);
    assert!(cosine_similarity(&[1.0, 0.0], &[0.0, 5.0]).abs() < 1e-12);
    assert!((cosine_similarity(&[1.0, 1.0], &[2.0, 2.0]) - 1.0).abs() < 1e-12);
    assert!((cosine_similarity(&[1.0, 0.0], &[-3.0, 0.0]) + 1.0).abs() < 1e-12);
    // Empty and zero-norm inputs degrade to 0.
    assert_eq!(cosine_similarity(&[], &[]), 0.0);
    assert_eq!(cosine_similarity(&[0.0, 0.0], &[1.0, 2.0]), 0.0);
}

#[test]
fn contradictory_candidate_has_phase_near_pi() {
    let mut index = QuantumRagIndex::new(2).phase_kickback(false);
    index.add(RagCandidate::new("c", "contradicted", vec![1.0, 0.0]).with_contradiction(1.0));
    let result = index.search(&[1.0, 0.0], 1, 1).unwrap();
    let phase = result.selected[0].phase;
    assert!(
        (phase - std::f64::consts::PI).abs() < 1e-9,
        "expected phase ~= pi, got {phase}"
    );
}

#[test]
fn interference_suppresses_high_sim_contradiction() {
    // Two contradicted chunks plus several coherent ones, all with identical
    // (maximal) cosine similarity to the query. Plain cosine cannot separate
    // them; interference must rank a relevant chunk above the contradicted one.
    let q = vec![1.0, 0.0, 0.0];
    let mut index = QuantumRagIndex::new(3).interference_rounds(4);
    for i in 0..5 {
        index.add(RagCandidate::new(format!("good_{i}"), "coherent", q.clone()));
    }
    // The contradicted chunk has the SAME embedding (cosine = 1.0).
    index.add(
        RagCandidate::new("bad", "contradicted but high-sim", q.clone()).with_contradiction(1.0),
    );

    let result = index.search(&q, 6, 3).unwrap();

    // Baseline cosine ties everything at 1.0 — "bad" can sit at the top.
    // Interference must push "bad" to the bottom.
    let good_score = result
        .selected
        .iter()
        .find(|s| s.id.starts_with("good"))
        .map(|s| s.score)
        .unwrap();
    let bad_score = result
        .selected
        .iter()
        .find(|s| s.id == "bad")
        .map(|s| s.score)
        .unwrap();
    assert!(
        good_score > bad_score,
        "relevant ({good_score}) must outrank contradicted ({bad_score})"
    );
    assert_ne!(result.selected[0].id, "bad");
    assert!(result.selected.last().unwrap().id == "bad");
}

#[test]
fn empty_index_is_handled() {
    let index = QuantumRagIndex::new(4);
    let err = index.search(&[1.0, 0.0, 0.0, 0.0], 5, 1);
    assert!(err.is_err());
}
