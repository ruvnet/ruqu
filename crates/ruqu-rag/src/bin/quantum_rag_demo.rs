//! Demo: cosine-weak vs interference-strong retrieval.
//!
//! A hand-written 6-document corpus answers the question "What is the current
//! recommended daily water intake?" One document (`d_old`) is highly similar to
//! the query but **outdated and contradicted** — plain cosine puts it on top, a
//! plausible-but-wrong answer. A well-cited, current document (`d_current`)
//! wins under interference reranking.
//!
//! Run with:
//! ```text
//! cargo run -p ruqu-rag --bin quantum_rag_demo
//! ```

use ruqu_rag::{
    baseline, context_precision_at_k, contradiction_rate_at_k, cosine_similarity, mrr, ndcg_at_k,
    recall_at_k, QuantumRagIndex, RagCandidate, Relevance,
};
use std::collections::HashSet;

/// Embedding dimension for the toy corpus.
const DIM: usize = 6;

fn main() -> anyhow::Result<()> {
    // Toy "embeddings". Dimensions (loosely): [water-intake, hydration, health,
    // outdated-myth, nutrition, exercise]. The query is about current water
    // intake guidance.
    let query = vec![1.0, 0.8, 0.5, 0.0, 0.2, 0.0];

    // d_old: the HIGHEST cosine to the query (nearly parallel) — it is the
    // outdated "8 glasses a day" myth, contradicted by current guidance, low
    // recency and trust. Plain cosine therefore ranks it #1: a plausible but
    // weak/wrong answer.
    let d_old = RagCandidate::new(
        "d_old",
        "You must drink exactly 8 glasses (2L) of water every day, no matter what.",
        vec![1.0, 0.8, 0.5, 0.0, 0.2, 0.0],
    )
    .with_contradiction(0.95)
    .with_recency(0.2)
    .with_source_trust(0.5);

    // d_current: the correct, well-cited, current guidance. Slightly lower raw
    // cosine than d_old, but coherent, fresh, and trusted.
    let d_current = RagCandidate::new(
        "d_current",
        "Current guidance: water needs vary by body size, activity and climate; \
         drink to thirst, roughly 2.7-3.7L total fluids/day from all sources.",
        vec![0.97, 0.82, 0.5, 0.0, 0.22, 0.04],
    )
    .with_recency(1.0)
    .with_source_trust(0.98)
    .with_graph_proximity(1.0);

    // d_support: corroborates the current answer (coherent, no contradiction).
    let d_support = RagCandidate::new(
        "d_support",
        "Health authorities note individual hydration needs differ; thirst is a \
         reliable guide for most healthy adults.",
        vec![0.9, 0.78, 0.52, 0.0, 0.2, 0.0],
    )
    .with_recency(0.9)
    .with_source_trust(0.95);

    // Distractors: clearly lower similarity to the query (little overlap with
    // the water-intake axes).
    let d_exercise = RagCandidate::new(
        "d_exercise",
        "Endurance athletes should plan electrolyte intake during long sessions.",
        vec![0.05, 0.1, 0.3, 0.0, 0.2, 0.95],
    );
    let d_nutrition = RagCandidate::new(
        "d_nutrition",
        "A balanced diet supplies most vitamins without supplementation.",
        vec![0.0, 0.05, 0.4, 0.0, 0.95, 0.1],
    );
    let d_unrelated = RagCandidate::new(
        "d_unrelated",
        "The history of aqueducts spans Roman engineering and modern utilities.",
        vec![0.0, 0.0, 0.15, 0.95, 0.05, 0.1],
    );

    let mut index = QuantumRagIndex::new(DIM).interference_rounds(4);
    index.add_many(vec![
        d_old.clone(),
        d_current.clone(),
        d_support.clone(),
        d_exercise.clone(),
        d_nutrition.clone(),
        d_unrelated.clone(),
    ]);

    println!("=== Quantum RAG demo: cosine-weak vs interference-strong ===\n");
    println!("Question: What is the current recommended daily water intake?\n");

    // (a) Plain cosine top-3.
    let mut cosine: Vec<(&RagCandidate, f64)> = index
        .candidates
        .iter()
        .map(|c| (c, cosine_similarity(&query, &c.embedding)))
        .collect();
    cosine.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    println!("(a) PLAIN COSINE top-3  [picks the outdated, contradicted doc]:");
    for (rank, (c, sim)) in cosine.iter().take(3).enumerate() {
        let flag = if c.id == "d_old" { "  <-- WEAK/WRONG WINNER" } else { "" };
        println!("    {}. {:<11} cos={:.4}{}", rank + 1, c.id, sim, flag);
        println!("       {}", short(&c.text));
    }
    println!();

    // (b) Interference top-3.
    let result = index.search(&query, 3, 42)?;
    println!("(b) INTERFERENCE top-3  [coherent, well-cited doc wins]:");
    for (rank, s) in result.selected.iter().enumerate() {
        let flag = if s.id == "d_current" {
            "  <-- CORRECT WINNER"
        } else {
            ""
        };
        println!(
            "    {}. {:<11} score={:.4} phase={:.3}rad{}",
            rank + 1,
            s.id,
            s.score,
            s.phase,
            flag
        );
        println!("       {}", short(&s.text));
    }
    println!();

    // (c) Gate decision.
    println!("(c) Coherence-gate decision: {}", result.gate.as_str());
    println!();

    // (d) Full collapse receipt JSON.
    println!("(d) Collapse receipt (auditable provenance):");
    println!("{}", result.receipt.to_json());

    println!();
    println!(
        "Summary: cosine top-1 = {}  vs  interference top-1 = {}",
        cosine[0].0.id, result.selected[0].id
    );

    // (e) Credible metric table (recommendation #2 of the SOTA report): cosine
    //     (weak floor) vs a SIMULATED strong supervised reranker vs
    //     interference, on the same toy corpus. The strong baseline is a noisy
    //     relevance oracle standing in for a cross-encoder/ColBERTv2 — NOT a
    //     real model. See docs/research/sota-landscape.md and the eval module.
    print_metric_table(&index, &query)?;

    Ok(())
}

/// Print the cosine-vs-strong-vs-interference metric table on the demo corpus.
fn print_metric_table(index: &QuantumRagIndex, query: &[f64]) -> anyhow::Result<()> {
    // Graded relevance + contradiction set for the toy corpus. d_current /
    // d_support are the grounded answer; d_old is contradicted-but-high-cosine;
    // the rest are distractors.
    let relevance = Relevance::from_pairs([
        ("d_current", 3.0),
        ("d_support", 2.0),
        ("d_old", 2.0), // topically relevant, but contradicted (below)
    ]);
    let contradicted: HashSet<String> = ["d_old".to_string()].into_iter().collect();

    let cosine = baseline::cosine_ranking(query, &index.candidates);
    let strong = baseline::supervised_ranking(&index.candidates, &relevance, 0.4, 1234);
    let interference: Vec<String> = index
        .search(query, index.candidates.len(), 42)?
        .selected
        .into_iter()
        .map(|s| s.id)
        .collect();

    let k = 3;
    println!("\n(e) Credible metrics (cosine=weak floor, supervised=SIMULATED strong):");
    println!(
        "    {:<14} {:>8} {:>10} {:>7} {:>12} {:>11}",
        "reranker", "nDCG@3", "Recall@3", "MRR", "ctx-prec@3", "contra@3"
    );
    for (name, r) in [
        ("cosine(weak)", &cosine),
        ("supervised(str)", &strong),
        ("interference", &interference),
    ] {
        println!(
            "    {:<14} {:>8.3} {:>10.3} {:>7.3} {:>12.3} {:>11.3}",
            name,
            ndcg_at_k(r, &relevance, k),
            recall_at_k(r, &relevance, k),
            mrr(r, &relevance),
            context_precision_at_k(r, &relevance, &contradicted, k),
            contradiction_rate_at_k(r, &contradicted, k),
        );
    }
    println!(
        "    Note: the simulated strong baseline is NOT a real cross-encoder; \
         interference's\n    edge is contradiction suppression (ctx-prec / contra), \
         not raw nDCG."
    );

    Ok(())
}

/// Trim a doc body for compact display.
fn short(text: &str) -> String {
    let collapsed: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.len() > 90 {
        format!("{}...", &collapsed[..90])
    } else {
        collapsed
    }
}
