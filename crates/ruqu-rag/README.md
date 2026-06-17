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

## Running ADR-258 §22 Test 1 (retrieval coherence)

```bash
cargo test -p ruqu-rag --test rag_tests test1_retrieval_coherence_faithfulness -- --nocapture
```

The test builds **200 candidates**: 20 RELEVANT (high cosine, no
contradiction), 20 CONTRADICTORY (**also** high cosine, so plain cosine ranks
them high, but `contradiction ≈ 1.0` so phase ≈ `π`), and 160 DISTRACTORS (low
similarity). "Top-10 citation faithfulness" is the fraction of the top-10 that
are RELEVANT. The test asserts the interference reranker beats plain cosine by
at least 0.10 (10 percentage points). Measured: **cosine ≈ 0.40 vs interference
≈ 1.00**.

## Public API

- `cosine_similarity(a, b) -> f64` — safe cosine (empty / zero-norm → `0`).
- `RagCandidate` — `new(id, text, embedding)` + `with_*` builders.
- `QuantumRagIndex` — `new(dim)`, `.interference_rounds(n)`, `.phase_kickback(b)`,
  `.add(..)`, `.add_many(..)`, `.search(query, k, seed)`.
- `SearchResult`, `ScoredCandidate`.

[`ruqu-possibility`]: ../ruqu-possibility
```
