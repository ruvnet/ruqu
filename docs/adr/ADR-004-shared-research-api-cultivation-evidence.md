# ADR-004: Shared Rust research API, bounded cultivation and RVF evidence

Date: 16 October 2026

Status: Accepted

## Context

The first research workflow established ideal distillation, sparse observable propagation and bounded decoding references. Extending it to a browser interface and physical circuit-level cultivation creates two risks: divergent numerical implementations and conflating software validation with hardware or decoded logical performance.

The recent Clifford-stabilizer paper supplies a concrete small cultivation-stage construction that can be independently realized with the existing dense simulator. This is a useful validation target without first implementing the paper's new polynomial simulation algorithm.

## Decision

1. Use `ruqu_core::research_api::run_json` as the shared numerical entrypoint. Native stdin/stdout, Node CLI and browser `research_run_json` adapters invoke the same Rust implementations. JavaScript provides presentation and request validation, not alternate research mathematics.
2. Expose strict bounded JSON operations for magic state analysis, sparse Pauli propagation, hypergraph decoding and cultivation. Reject unknown fields, nonfinite values, invalid decimal-string seeds or masks, oversized requests and exhausted budgets.
3. Implement the published f=3 cultivation-stage construction as a dense 16-qubit trajectory model: 13 data qubits, 3 cat ancillas and exactly two logical verification rounds. Name the protocol and specify the internal syndrome-gadget ordering choice.
4. Keep escape, final correction/decoding and logical-gate consumption outside the modeled stage. Report accepted/rejected attempts, stage rejection, full-state overlap, codespace probability and joint codespace/HXY-minus probability separately.
5. Provide an animated [browser Explorer](https://ruvnet.github.io/ruqu/explorer/) with local WebAssembly workers, bounded batches, explicit assumptions, pause-motion controls and JSON evidence export.
6. Package research evidence through official RVF WASM binary storage with a required hash-bound JSON sidecar. Imported evidence remains untrusted for backend promotion; retrieval is advisory.

This decision extends ADR-003. A dense circuit realization is now available, while the full Surti and Clifford-stabilizer algorithms and Google's experiment remain outside the implemented scope.

## Validation evidence and interpretation

The exhaustive single-fault audit evaluates 2,485 mechanisms at 253 physical locations. It finds zero accepted joint codespace/HXY-minus probabilities above 1e-10 under ideal final error detection. Accepted raw infidelity can remain because physical errors need not be encoded logical flips. This first-order result does not establish fault distance three, which requires two-fault analysis and/or proof.

An independent NumPy oracle checks 29 ideal and selected-fault cases against the Rust implementation; the recorded maximum absolute difference is approximately 5.11 × 10^-15. It provides independent arithmetic validation for those cases, not an independent exhaustive fault-distance certificate. [Recorded evidence](../research/cultivation-independent-evidence.json).

The syndrome measurement CNOT order is ascending data index inside each check. The source paper refers to another syndrome-gadget construction without enumerating that internal ordering in text. Consequently this is a concrete realization of the published construction; equivalence to the authors' full noisy curves is unestablished. [Protocol details](../research/cultivation-protocol.md).

## Evidence trust boundary

Evidence identity includes circuit digest, noise model, seed, backend, environment and computation budget. SHA-256 hashes detect tampering, but do not authenticate a claimed benchmark. Imported RVF records carry `trustedForPromotion: false`. Backend promotion requires distinct paired native holdouts that the current process has locally validated, plus improvement and regression checks. Browser exports are request/result JSON and do not automatically qualify as native measurements. [Memory contract](../research/research-memory.md).

## Consequences

One implementation reduces cross-runtime drift and supports direct native/WASM comparison. A worker protects browser interaction, but dense trajectories still have exponential memory cost and runtime limits. The 16-qubit amplitude vector occupies 1 MiB before references and bookkeeping.

The highest risk is presenting raw state infidelity or a first-order fault audit as a decoded logical error curve or distance proof. Keep the output metric names and exclusions visible; extend fault-combination analysis and model escape separately before such comparisons.

## Acceptance

Run shared requests through native and WASM paths and compare structured results. Verify clean-state preparation, code checks, count conservation, explicit limits and null output metrics when no state is accepted. Reproduce the independent oracle evidence and single-fault audit. Validate evidence integrity, advisory retrieval and rejection of untrusted promotion. Open the Explorer on desktop and mobile, execute all research views, and export request/result JSON.

No npm publication or hardware execution is implied by this decision.
