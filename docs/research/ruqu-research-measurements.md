# Measured Rust/WASM research memory example

`ruqu-research.rvf` contains two real software execution records for the CLI's ideal 15-to-1 independent-input-Z model, with seeds 42 and 43 and 1,000 attempts per run. It is an actual binary RVF store produced by the pinned official RVF WASM implementation; `ruqu-research.rvf.json` is its required integrity-bound metadata sidecar. `ruqu-research-measured.json` contains the records in readable form, and `ruqu-research-measurement-report.json` preserves the full CLI outputs and roundtrip validation.

These are measured Rust/WASM software executions, not synthetic unit-test fixtures. The modeled input faults are synthetic and the protocol model assumes ideal Clifford operations and checks. This example is not a physical cultivation reproduction or hardware experiment.

## Results

| Seed | Attempts | Accepted | Rejected | Fresh process elapsed | CLI run elapsed | Process maxRSS upper bound |
| :--- | ---: | ---: | ---: | ---: | ---: | ---: |
| 42 | 1,000 | 869 | 131 | 84.48 ms | 7.55 ms | 42,336,256 bytes |
| 43 | 1,000 | 865 | 135 | 79.99 ms | 10.58 ms | 42,205,184 bytes |

Both records use the model's exact conditional output error `0.0000360876839653233` and exact acceptance probability `0.8600903336704236`. The sampled error count is zero in these small runs; that observation is not a zero-error guarantee. The complete sample confidence intervals remain in the raw reports.

Fresh process elapsed is measured with the parent monotonic clock and includes Node startup, actual invocation of the CLI's `run` implementation, and process shutdown. CLI run elapsed is measured within the fresh process. The memory field is `process.resourceUsage().maxRSS * 1024`, a cumulative process high-water upper bound that includes Node and CLI/WASM allocations. It is not isolated simulator memory. Resource costs in the model output use explicitly supplied illustrative defaults; they are expected resource estimates, not measured hardware costs.

Both records are development examples using the same backend. They establish no backend comparison, holdout improvement, hardware performance, or promotion eligibility.

## Reproduce

The script verifies the complete WASM source/distribution manifest before executing either run, records source and binary hashes from that manifest, runs two isolated measurement workers, and verifies exact-key lookup plus real RVF retrieval after import.

```bash
node scripts/measure-research-memory.cjs --output-dir /tmp/ruqu-measured-example
node cli/bin/cli.js research-memory import --input /tmp/ruqu-measured-example/ruqu-research.rvf
```

Use a new output directory because artifacts are created exclusively and existing files are never overwritten. To inspect the checked-in artifact:

```bash
node cli/bin/cli.js research-memory import --input docs/research/ruqu-research.rvf
```

The binary vector store is 258 bytes. Both rows have the same structural retrieval features, so either may appear first in nearest-neighbor retrieval. Only the complete exact target key, including the seed, distinguishes exact evidence identity. The script verifies matching record digests without assuming tie ordering.

Acceptance: a fresh run exports two records, imports identical records with `trustedForPromotion: false`, matches both exact cache keys, and finds each digest at zero structural distance through actual RVF WASM retrieval. No configuration is modified.
