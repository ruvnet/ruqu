# ruqu-rag — Interference Reranking for RAG

Phase 2 of **ADR-258 ("Interference Reranking")**. A drop-in replacement for
plain cosine reranking in retrieval-augmented generation, built on the
possibility-field runtime in [`ruqu-possibility`].

## The problem with cosine reranking

Cosine similarity ranks a candidate purely by how close its embedding sits to
the query. It is **blind to structure**: an outdated, contradicted, or
low-trust chunk that happens to be semantically close to the query scores just
as high as a fresh, well-cited, coherent chunk. A confident-sounding but *wrong*
answer can end up cited first.

## The interference reranker

Each candidate is mapped to a **complex amplitude**:

- the **magnitude** encodes *how strongly* the candidate supports an answer;
- the **phase** encodes *which* answer it supports.

When the field interferes, candidates pointing the same way **constructively
amplify**, while contradicted candidates (phase near `π`) **destructively
cancel**. Coherent evidence wins even when a contradictory chunk has a higher
raw cosine, and every decision is auditable via a `CollapseReceipt`.

### Amplitude / phase mapping

For query `q` and candidate `c`:

```text
sim         = clamp(cosine_similarity(q, c.embedding), 0, 1)
magnitude_0 = sim * c.source_trust * c.recency * c.graph_proximity
phase_0     = π * c.contradiction               (+ 0.1 * c.novelty  if phase_kickback)
```

- `magnitude_0` is a **non-negative, monotonic** combiner: more similar, more
  trusted, fresher, and graph-closer all raise the magnitude.
- `phase_0` rotates a fully-contradicted candidate (`contradiction = 1.0`) by
  `π` so it sits opposite the coherent bulk and cancels under interference. The
  optional novelty kickback applies a small (`<= 0.1` rad) perturbation so
  near-duplicate evidence does not perfectly stack.

Each candidate also carries an `EvidenceReceipt` (`source = id`,
`trust = source_trust`, payload = the chunk text bytes).

### Interference rounds

For `interference_rounds` iterations, the field is refined:

1. compute the field's **net (resultant) complex amplitude**
   `R = Σ aₖ e^{iφₖ}`;
2. for each candidate, measure its **alignment** with `R` — the projection of
   its unit amplitude onto the unit resultant, i.e. `cos(Δphase) ∈ [-1, 1]`;
3. rescale its magnitude by `(1 + alignment)` (clamped at `0`) — aligned
   (constructive) candidates amplify, opposed (destructive) candidates decay;
4. renormalize.

This is the ADR-258 "interference scoring" primitive applied at the field level.
It is fully deterministic: the same field always yields the same reranking.

### Search

`QuantumRagIndex::search(query, k, seed)` returns a `SearchResult`:

- `selected` — top-`k` `ScoredCandidate`s by post-interference probability;
- `receipt` — a `CollapseReceipt` whose `selected_id` is the interference top-1,
  carrying the reranked field's coherence, entropy, gate decision and field hash;
- `gate` — the `CoherenceGate` decision (`PERMIT` / `DEFER` / `DENY`);
- `baseline_cosine_top_k` — the plain-cosine top-`k` ids, for comparison.

## Usage

```rust
use ruqu_rag::{QuantumRagIndex, RagCandidate};

let mut index = QuantumRagIndex::new(/* dim */ 3).interference_rounds(4);
index.add(RagCandidate::new("good1", "fresh, cited", vec![1.0, 0.0, 0.0]));
index.add(RagCandidate::new("good2", "also cited", vec![1.0, 0.0, 0.0]));
index.add(
    RagCandidate::new("bad", "outdated, contradicted", vec![1.0, 0.0, 0.0])
        .with_contradiction(1.0),
);

let result = index.search(&[1.0, 0.0, 0.0], 3, 42).unwrap();
assert_eq!(result.selected.last().unwrap().id, "bad"); // suppressed
```

## Running the demo

```bash
cargo run -p ruqu-rag --bin quantum_rag_demo
```

A 6-document corpus answers "What is the current recommended daily water
intake?" One doc (`d_old`, the "8 glasses a day" myth) is the **highest cosine**
match but is outdated and contradicted — plain cosine picks it #1. Under
interference the coherent, well-cited `d_current` wins and `d_old` drops out of
the top-3. The demo prints the cosine top-3, the interference top-3, the gate
decision, and the full `CollapseReceipt` JSON.

## Evaluation — credible metrics, not "+X% over cosine"

> **Cosine is the weak floor.** "+10% faithfulness over cosine" is *not* a
> meaningful claim: cosine is the universally-beaten baseline in the reranking
> literature ([SOTA report](../../docs/research/sota-landscape.md),
> recommendation #2). A credible result must report **standard IR metrics**
> against a **strong** baseline and prove **contradiction suppression** on a
> grounding axis.

The [`eval`](src/eval.rs) module provides pure, unit-tested ranking metrics over
a ranked list of ids given graded/binary relevance labels:

- `ndcg_at_k` / `dcg_at_k` — graded relevance, log2 discount (nDCG ∈ `[0,1]`);
- `recall_at_k`, `precision_at_k`, `mrr`, `average_precision` (MAP);
- `context_precision_at_k` — the **faithfulness proxy**: fraction of the top-`k`
  that are *relevant **and** not contradicted* (RAGAS-style grounding);
- `contradiction_rate_at_k` — fraction of the top-`k` that are contradicted
  (the direct suppression metric; lower is better).

The `baseline` submodule supplies both opponents:

- `cosine_ranking(query, candidates)` — the **weak floor**;
- `supervised_ranking(candidates, relevance, noise, seed)` — a **SIMULATED**
  strong supervised reranker. It ranks by the (noisy) graded relevance, the best
  a perfectly-trained cross-encoder could do. **It is not a real cross-encoder**
  — it is a deterministic stand-in for a *fair, strong* comparison. A generic
  supervised reranker is trained for *topical relevance*, not *answer
  grounding*, so it ranks a contradicted-but-on-topic chunk near the top; that
  is exactly where interference differentiates.

### Benchmark

```bash
cargo test -p ruqu-rag --test eval_tests credible_reranking_benchmark -- --nocapture
```

Builds **200 candidates** (20 RELEVANT, 20 CONTRADICTORY-but-high-cosine-and-
topically-relevant, 160 DISTRACTORS) with graded relevance labels and a
contradiction set, then reports the table for **cosine vs simulated-strong vs
interference** (measured):

| reranker            | nDCG@10 | Recall@10 |   MRR | ctx-prec@10 | contra@10 |
|---------------------|--------:|----------:|------:|------------:|----------:|
| cosine (weak)       |   0.755 |     0.250 | 1.000 |       0.400 |     0.600 |
| supervised (strong) |   1.000 |     0.250 | 1.000 |       0.600 |     0.400 |
| **interference**    |   0.780 |     0.250 | 1.000 |   **1.000** | **0.000** |

The asserted, **honest** claims:

- **(a)** interference beats the *weak cosine floor* on nDCG@10 **and**
  context-precision@10;
- **(b)** vs the *strong* baseline, interference's advantage is specifically
  **contradiction suppression** — higher context-precision@10 (1.000 vs 0.600)
  and lower contradiction-rate@10 (0.000 vs 0.400) — **even though the strong
  supervised baseline wins raw nDCG@10** (1.000 vs 0.780). We do *not* claim an
  nDCG win; the test asserts the strong baseline is not beaten on nDCG, to keep
  the comparison honest.

### Legacy test (ADR-258 §22 Test 1)

```bash
cargo test -p ruqu-rag --test rag_tests test1_retrieval_coherence_faithfulness -- --nocapture
```

The same 200-candidate setup using a simple top-10 faithfulness fraction
(cosine ≈ 0.40 vs interference ≈ 1.00). Prefer the metric-based benchmark above
for any external comparison.

## Limitations / future validation

These metrics are **offline implementations and a synthetic harness**, not
external validation. The strong baseline is *simulated*, not measured. To make a
publishable claim, validate against the real SOTA named in the
[SOTA report](../../docs/research/sota-landscape.md):

- **Strong baselines:** a real cross-encoder (BGE-reranker-v2-m3, ~0.66 avg
  nDCG@10 on BEIR) and **ColBERTv2** (~39.7% MRR@10 on MS MARCO, ~50 nDCG@10 on
  BEIR) — *not cosine*. Also BM25 + RRF.
- **Datasets:** **BEIR** / **MS MARCO** for nDCG@10 + Recall@k; report latency
  for top-100/200 (cross-encoders run ~5–10 ms).
- **Faithfulness / grounding:** **RAGAS** faithfulness + context-precision.
- **Contradiction suppression:** **RGB-counterfactual**, **FaithEval**,
  **RAMDocs** (MADAM-RAG sets a bar of +15.8% on FaithEval).

The interference reranker's *expected* niche, supported by the prior art (the
Quantum Probability Ranking Principle, Zuccon & Azzopardi 2009/2010), is the
**contradiction-suppression / grounding** axis — not necessarily raw topical
nDCG, where a trained supervised reranker is strong.

## Public API

- `cosine_similarity(a, b) -> f64` — safe cosine (empty / zero-norm → `0`).
- `RagCandidate` — `new(id, text, embedding)` + `with_*` builders.
- `QuantumRagIndex` — `new(dim)`, `.interference_rounds(n)`, `.phase_kickback(b)`,
  `.add(..)`, `.add_many(..)`, `.search(query, k, seed)`.
- `SearchResult`, `ScoredCandidate`.
- `eval` module (additive): `Relevance`, `ndcg_at_k`, `dcg_at_k`,
  `recall_at_k`, `precision_at_k`, `mrr`, `average_precision`,
  `context_precision_at_k`, `contradiction_rate_at_k`, and `baseline::{cosine_ranking,
  supervised_ranking}`.

[`ruqu-possibility`]: ../ruqu-possibility
```
