# ADR-003: Bounded magic state analysis and correlated decoding research

Date: 16 October 2026

Status: Accepted for implementation

## Context

Recent cultivation, structured magic state simulation and hypergraph decoding research creates useful opportunities for ruqu. Experimental fidelity and favorable asymptotic results alone do not establish an implementation, practical speedup or hardware advantage. We need independently checkable primitives before reproducing a complete protocol.

Primary sources, dates and scientific caveats are recorded in [the research review](../research/2026-10-16-quantum-advances.md).

## Decision

Add a bounded source-checkout research workflow with three independently testable foundations:

1. An ideal 15-to-1 magic state distillation reference, with exact finite-pattern enumeration, deterministic sampling, and expected resource costs that include rejected attempts.
2. Sparse Heisenberg Pauli observable propagation with explicit term budgets. Supported Clifford transformations, T/T-dagger branching, Pauli noise and postselection provide tractable controls for selected circuits.
3. A bounded hypergraph syndrome decoder for small correlated-fault examples, with explicit search bounds and syndrome consistency checks.

Expose the factory through the Node source CLI and native Rust examples. Keep native-only research functionality distinct from published npm and WASM exports. Preserve the existing observer consistency laboratory and its evidence boundaries.

Improve discoverability with a descriptive README, runnable source commands and native SVG introductions, walkthroughs and diagrams. SVGs must remain readable without animation, provide accessible text alternatives, support reduced motion and embed no scripts or external assets.

## Model rules

1. Ideal distillation input error is not physical gate error. Ideal operations, independent stochastic inputs and conditional output probabilities must be explicit.
2. Expected resources include failure probability. An accepted-state cost must not use per-attempt resources without dividing by yield.
3. Sparse propagation is exact algebra for its supported model, subject to floating-point precision and resource limits. Budget exhaustion is an error, not an approximate result.
4. Decoder accuracy and optimality claims must name the completed search conditions. No Tesseract-equivalence or large-code latency claim follows from small-instance correctness.
5. Google's experimental fidelity and transfer result are separate evidence. The implemented distillation reference is not a cultivation reproduction.
6. Structured propagation is motivated by recent research but does not implement the full Surti or Clifford-stabilizer algorithms.

## Consequences

This approach delivers a useful resource planning and verification foundation at low compute cost while keeping the scientific boundary inspectable. It adds separate implementations in Rust and JavaScript, requiring agreement tests for duplicated factory mathematics.

The largest risk is treating a favorable ideal-model estimate as a physical performance prediction. The fix is an explicit progression to circuit-level faults, correlation and leakage stress tests, protocol circuits and independent transfer validation.

## Acceptance

Source commands execute; exact probabilities normalize; seeded results repeat; sparse observables match independent small dense references; decoder outputs satisfy syndromes; configured bounds reject unsupported workloads; documentation does not claim unmeasured hardware fidelity or speedups.

No full cultivation circuit, upstream decoder port, publication or hardware deployment is implied by this decision.
