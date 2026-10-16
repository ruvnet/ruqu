# Measured research memory with real RVF storage

ruqu records bounded simulator evidence and retrieves structurally similar experiments using the official RuVector RVF WASM runtime. It does not invoke a provider, perform a paid experiment, retrain a model, or automatically switch a simulator backend.

## Implementation and dependencies

The integration pins [`@ruvector/rvf-wasm@0.1.9`](https://www.npmjs.com/package/@ruvector/rvf-wasm) and directly wraps its documented C ABI. The public package has no native binary dependency and is approximately 47 KB unpacked. The inspected package exports `rvf_store_create`, `rvf_store_ingest`, `rvf_store_export`, `rvf_store_open`, `rvf_store_query`, count/dimension inspection, and allocation functions.

The upstream [`@ruvector/rvf` SDK](https://github.com/ruvnet/ruvector/tree/main/npm/packages/rvf) uses the same ABI. Version 0.3.4 explicitly rejects nonempty metadata objects: durable per-vector metadata is not supported through that SDK API. Therefore this implementation exports two files:

1. `memory.rvf`: an actual binary store created by the official RVF WASM runtime, with eight-dimensional fp32 retrieval vectors and numeric vector IDs.
2. `memory.rvf.json`: a canonical JSON sidecar containing the full evidence records, exact identity hashes, numeric ID ordering, and the RVF byte hash.

The JSON sidecar is not itself an RVF file. Both files are required for interchange. This does not claim the WASM store export is interchangeable with every native RVF runtime version. Its tested portability contract is official WASM export followed by official WASM open.

## Evidence contract

`createEvidence` receives the following structure. Values below illustrate the contract only; they are not measurements.

```javascript
const { createEvidence } = require('./cli/src/research-memory.cjs');
const evidence = createEvidence({
  target: {
    circuitSha256: circuitHash,    // hash the exact canonical circuit bytes
    noise: noiseConfiguration,    // include every noise parameter and model version
    seed: seed,                    // safe nonnegative integer or canonical decimal u64 string
    backend: backendVersion,
    environment: { machine: machineId, platform: process.platform, architecture: process.arch },
    features: { qubits, gates, tGates, measurements },
    budget: { maxRuntimeMs, maxMemoryBytes, maxOutputError, minAcceptance }
  },
  metrics: { runtimeMs, peakMemoryBytes, outputError, attempts, accepted, rejected },
  provenance: {
    kind: 'native-measured',
    runId: uniqueRunId,
    toolVersion: version,
    sourceSha256: sourceHash,
    measuredAt: new Date().toISOString(),
    partition: 'development' // or a predeclared holdout
  }
});
```

Measure runtime and memory in the actual simulator process. Record the error definition and uncertainty in additional JSON fields; `outputError` must come from the actual protocol evaluation or independent exact reference. Do not substitute a gate count estimate for measured memory, or a preparation infidelity for logical output error. All optional and additional target fields are included in the identity hash. Seeds must be safe nonnegative JavaScript integers or canonical decimal u64 strings. Use strings above `Number.MAX_SAFE_INTEGER`; unsafe numeric seeds are rejected to prevent rounding collisions. A different seed, complete noise object, budget, source/backend version or machine environment needs a different exact key. Include source hashes and compiler/build identifiers in `target.environment` when comparing optimized builds. Provenance hashes authenticate no external source and are outside the exact target key; putting a changed source hash only in provenance does not change that key.

`attempts = accepted + rejected` is checked. All probabilities are bounded to [0,1], and numerical data must be finite. Evidence expires after 30 days by default. Future timestamps, duplicate run IDs, altered records, stale evidence and over-budget candidates are rejected or excluded. A record's `sourceSha256` identifies code provenance; it does not prove that the experiment actually ran.

Kinds `reference-exact` and `synthetic-test` can be validated but are excluded from native measured recommendations and RVF exports. Unit tests explicitly use synthetic fixtures to exercise the native-measured schema. Those fixtures are never research results.

## CLI walkthrough

From the repository checkout, after installing CLI dependencies:

```bash
node cli/bin/cli.js research-memory export --input measured-evidence.json --output memory.rvf
node cli/bin/cli.js research-memory import --input memory.rvf
node cli/bin/cli.js research-memory query --input memory.rvf --target target.json
node cli/bin/cli.js research-memory recommend --input measured-evidence.json --target target.json
```

`measured-evidence.json` is an array of records returned by `createEvidence`. The target file contains the complete target object. Exports use exclusive file creation and never overwrite an existing artifact. Input files are limited to 16 MiB each and 10,000 records; queries return at most 100 neighbors. The CommonJS module exports async `run` and `runCLI` for CLI integration, and can also be invoked directly with Node.

`lookupExact(records, target)` returns a fresh native measured record matching the complete target key or `null`. It is a deterministic historical evidence cache, not permission to treat an old runtime as a fresh benchmark.

## Advisory retrieval and bounded promotion

The eight retrieval features are log-scaled qubit/gate/T-gate/measurement counts, physical error rate, requested output error, minimum acceptance and log runtime budget. L2 retrieval runs in actual RuVector RVF WASM. These are hand-selected structural features, not learned embeddings or proof that two circuits are equivalent. Source, noise, seed and budget information stays in the sidecar and is checked separately.

`recommend` ranks validated measured records and reports `advisoryOnly: true`. `queryMemory` performs actual RVF nearest-neighbor retrieval and likewise preserves the requested backend. Similar experiments can suggest which backend to benchmark next; they never supply validated answers for a different circuit. A caller must not treat semantic neighbors as exact cache hits.

`checkPromotion` evaluates a proposed backend using at least two pairs of distinct, predeclared holdout seeds. Each pair must have identical full targets except backend, consistent candidate/baseline backend identities, unique run IDs and native measured provenance. The caller must pass `locallyValidatedEvidence: [{ runId, digest }]` from its own trusted execution ledger. Each digest must equal the complete approved record's SHA256 digest, not merely its run ID. String-only run ID approvals are rejected. Reusing an approved run ID with modified metrics and a recomputed digest is rejected because that digest is absent from the ledger. Do not construct this allowlist from imported files; imported JSON alone does not satisfy the trust requirement. Candidate output error cannot regress beyond the explicit tolerance, acceptance cannot decrease, budgets must be met and aggregate runtime must improve by at least 10% by default. An eligible result is still advisory and never modifies configuration. Two pairs are a minimum mechanical gate, not statistical significance; use enough repeated runs and confidence intervals for the workload before making performance claims.

## Integrity and trust boundaries

SHA256 binds each record and the manifest to the actual RVF bytes. Import verifies these hashes, the bounded shape, record freshness, record kinds and replay within the batch before opening the store. Query rechecks bytes to detect replacement between validation and retrieval. All WASM handles and allocations are released. This is integrity checking, not authenticated signing: someone who controls both files can replace records and recompute their hashes. Imported evidence is always marked `trustedForPromotion: false`; local paired validation is required. No claim is made about upstream witness-chain signatures or physical hardware accuracy.

Tests cover real official WASM export/open/query, full-target cache invalidation, tampered RVF and sidecar rejection, stale records, future timestamps, unsafe keys, replay, budget limits and paired holdout promotion gates:

```bash
node --test cli/tests/research-memory.test.cjs
```

Acceptance: export genuine measured records, reopen both files in a fresh process, reproduce the exact record hashes and retrieval identities, mutate one byte and observe rejection, then demonstrate that an imported recommendation cannot change the selected backend or satisfy promotion without locally validated paired holdouts.

## Measured software example

Run `node scripts/measure-research-memory.cjs` to measure two seeded Rust WASM factory runs and verify a real RVF roundtrip. [Recorded measurements](ruqu-research-measurement-report.json), [records](ruqu-research-measured.json), [RVF binary](ruqu-research.rvf) and its required [metadata sidecar](ruqu-research.rvf.json) are included. These development runs do not authorize backend promotion. RSS is a process high water bound; resource estimates describe the ideal model, not hardware.
