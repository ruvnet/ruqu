# Quantum advances relevant to ruqu

Reviewed: 16 October 2026. This document distinguishes primary research from independently implemented software. No paper's experimental fidelity, scaling result or speedup is a ruqu measurement.

## Research map

| Primary source | Verified finding | ruqu action and boundary |
| :--- | :--- | :--- |
| Rosenfeld et al., [Magic state cultivation on a superconducting quantum processor](https://arxiv.org/abs/2512.13908), 15 December 2025 | Reports fidelity 0.9999(1), 40-fold error reduction and 8% retention. | Account for rejection and distinguish preparation from code transfer. This is an experimental reference, not the implemented 15-to-1 protocol. |
| Surti, Daguerre and Kim, [Efficient simulation of logical magic state preparation protocols](https://arxiv.org/abs/2512.23799), PRX Quantum 7, 020329 (2026); revision 25 May 2026 | Specialized preparation circuits under Pauli noise permit simulation cost polynomial in qubits and target nonstabilizerness. | Supports investigating structured simulation. Sparse Pauli observable propagation is a standard independent baseline, not a reproduction of their algorithm. |
| Takada, Bartlett and Williamson, [Exact efficient simulation of noisy logical magic states using Clifford stabilizers](https://arxiv.org/abs/2609.16929), 15 September 2026 | Introduces a Clifford-stabilizer method for a supported family of noisy preparation protocols, with a distance-7 cultivation demonstration. | Dense f=3 circuit-stage realization, excluding escape and final decoder. The polynomial Clifford-stabilizer group algorithm and distance-7 result remain unimplemented. |
| Ruiz et al., [Unfolded distillation](https://www.nature.com/articles/s41534-026-01197-z), npj Quantum Information 12, 53, 20 February 2026 | Under specified strongly biased noise, reports a modeled 3 × 10^-7 logical error using 53 qubits and 5.5 correction rounds at 0.1% phase-flip rate and bias at least about 5 × 10^6. | Compare protocols only with hardware noise assumptions attached. No generic 53-qubit factory claim. |
| Aghababaie Beni, Higgott and Shutty, [Tesseract](https://arxiv.org/abs/2503.10988), 2025; [upstream implementation](https://github.com/quantumlib/tesseract-decoder) | Search-based decoding handles detector hypergraphs and correlated fault hypotheses. | Independent bounded small-instance hypergraph baseline. It is not Tesseract, a port, or evidence of equivalent accuracy or latency. |
| [Accelerating A*-Based Algorithms for Decoding Quantum Low-Density Parity-Check Codes](https://arxiv.org/abs/2609.10056), September 2026 | Studies accelerating decoding with belief propagation and search. | Future comparative benchmark. No imported speedup or implementation claim. |

## Shared numerical implementation and Explorer

Native Rust `ruqu_core::research_api::run_json`, Node CLI `research`, and browser `research_run_json` invoke the same Rust numerical implementation. The [animated Quantum Explorer](https://ruvnet.github.io/ruqu/explorer/) executes it in a browser worker and exports request/result JSON. Shared operations include ideal factory analysis, bounded dense cultivation, sparse Pauli propagation and hypergraph decoding. API budgets apply before expensive work; seeds and bit masks remain decimal strings across JavaScript boundaries.

The [cultivation protocol transcription](cultivation-protocol.md) identifies Table 3 and the relevant figures in arXiv:2609.16929v1. The dense 16-qubit implementation realizes its f=3 cultivation stage with an explicit internal syndrome-gadget ordering choice. Escape, final decoding and logical gate consumption are excluded. The new circuit capability does not imply implementing the paper's efficient Clifford-stabilizer algorithm.

An exhaustive single-fault audit covers 253 locations and 2,485 mechanisms with exact selected-branch weights. No accepted joint codespace/HXY-minus probability exceeded the numerical threshold. Ideal final error detection is part of that interpretation; two-fault analysis or a proof would still be required to certify distance three. Independently, a NumPy oracle compares 29 ideal and selected-fault cases with maximum absolute difference 5.11 × 10^-15. That separate oracle does not enumerate all single faults. [Machine-readable independent evidence](cultivation-independent-evidence.json).

## Cultivation is not distillation

The current exact factory reference is ideal 15-to-1 distillation with independent input errors and ideal operations. Enumerating 32,768 input patterns yields acceptance and conditional output error. It provides a compact oracle for seeded statistical checks and yield accounting. It does not simulate Google's color-code cultivation circuit.

The cultivation experiment's pre-transfer fidelity is not a post-transfer logical gate fidelity. Its distance-5 grafted state is limited by idle errors; conversion to a matchable code and real-time T-gate execution remain future steps in that work. See [the paper's grafting and outlook sections](https://arxiv.org/html/2512.13908v1). Treating all these stages as one error number would understate execution risk.

At 8% retention the expected attempts per accepted state is 12.5. This arithmetic ignores throughput limits, parallel factories, storage and transfer faults. The ideal distillation estimator's qubit-cycle and input-state estimates must likewise identify their accounting boundary.

## Implemented software scope

| Component | Checkable invariant | Limitation |
| :--- | :--- | :--- |
| Exact ideal 15-to-1 reference | Accepted and rejected probability sum to one; output error is conditional on acceptance. | Input stochastic errors only; distillation operations are ideal. |
| Seeded sampling | Repeated seed and parameters reproduce counts; statistical estimates can be compared to exact values. | Rare logical errors require large sample counts; zero observed errors is not zero error probability. |
| Sparse Pauli propagation | Supported expectations agree with independent dense calculations on small circuits. | T gates and postselection can grow the term count; configured branch budgets bound computation. |
| Bounded hypergraph decoding | Returned hypotheses reproduce the requested detector syndrome; small instances admit exhaustive comparison. | Search budget and instance size limit applicability; no real-time QEC performance claim. |

Sparse propagation supports H, S, CNOT, CZ and SWAP, branching for T and T-dagger, stochastic independent or correlated Pauli faults, Pauli postselection, and product inputs supported by the API. Exactness means no deliberate truncation of supported algebra; floating-point roundoff remains. A large supported Clifford circuit does not imply efficient arbitrary non-Clifford simulation.

## RVF evidence and governance

`research-memory` exports a genuine binary RVF store using official `@ruvector/rvf-wasm`, with a required JSON sidecar binding records and binary hashes. Exact evidence identities cover circuit digest, noise, seed, backend, environment and budgets. [The memory contract](research-memory.md) describes validation, integrity checks and advisory retrieval.

Integrity does not authenticate a measurement. Imported records carry `trustedForPromotion: false`; retrieval does not switch backends. Promotion requires distinct locally validated paired native holdouts and the configured improvement and regression gates. Browser JSON exports are not automatically native-measured promotion evidence.

## Validation and next experiments

1. Compare sparse observables against dense state vectors on supported small circuits, including non-Clifford gates and postselection. Fail explicitly when a configured computational bound is exceeded.
2. Compare factory sampling to exact enumeration; report attempted, accepted, rejected and accepted-error counts. Preserve the denominator for each probability.
3. Check decoder hypotheses against the syndrome and exhaustive small-instance references. Preserve a distinction between completed search and exhausted budget.
4. Extend cultivation validation beyond the existing dense f=3 stage: independently audit additional fault combinations, validate ordering choices, and model escape and decoding separately before comparing logical error curves.
5. Measure host, build profile, latency and memory before claiming improvement. Keep biased-noise assumptions attached to any comparison with unfolded distillation.

## Reproduce from source

```bash
node cli/bin/cli.js magic-factory --input-error 0.01 --shots 100000 --seed 42
node cli/bin/cli.js magic-factory --benchmark
cargo run -p ruqu-core --example magic_states
cargo run -p ruqu-core --example hypergraph_decoder
cargo run -p ruqu-core --example pauli_propagation
cargo run -p ruqu-core --release --example cultivation
cargo run -p ruqu-core --release --example cultivation -- --audit
python scripts/verify-cultivation.py
cargo test -p ruqu-core
```

Commands refer to this source checkout, not an asserted npm release. Runtime results, not paper headlines, are the acceptance evidence.
