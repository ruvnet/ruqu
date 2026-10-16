# Validation receipt

Verdict: ACCEPT for the experimental CPU correctness milestone only.
Production promotion and acceleration claims: INCONCLUSIVE.

Owner: Codex coordinator with separate specification, implementation, integration,
and evaluator roles. Ruflo coordination used @claude-flow/cli 3.25.6. The
implementation worker did not inspect QuaSARQ source or documentation. The
coordinator had previously read its documentation; no legal clean-room guarantee
is made. An independent agent reviewed the implementation after test authoring.

## Exact candidate

Base repository: eb127994aaf8cdb074ebcc3ff32c1a798b79efa8.
Initial candidate: 2bf774eb684531d3dfb000ceb7b0498f7a4b2828.
Tested candidate: 52a93363ac184d6f8b639ed99623ef963071a3d8.
The initial build exposed a crate-name mismatch between manifest and callers;
the manifest was corrected. No simulation algorithm change was needed after
oracle evaluation. Subsequent changes only add documentation and evidence.

## Results

Rust 1.77.2 (25ef9e3d8), isolated Linux ruOS executor:

* 7 independent oracle tests passed; 7 CLI tests passed; zero failures.
* 30 development plus 10 separate confirmation seeds, five widths, 96 generated
  steps each: 19,200 operations checked against a dense complex oracle.
* Signed stabilizer and symplectic checks after every generated step.
* Analytic GHZ and phase-rich sparse-state cases at 31/32/33/255/256/257 qubits.
* CLI tests verify 30-seed Bell correlations, replay, reset, malformed input,
  bounds and no partial output after late parse errors.
* Zero-dimensional state, size overflow and 64 MiB allocation-cap tests passed.
* Clippy 1.90.0 on library and binary passed with warnings denied.
* No new dependencies or unsafe code. Independent source review found no blocking
  phase or memory-safety defect. This is not a comprehensive security audit.

Process-inclusive release diagnostic, 257-qubit GHZ plus 257 measurements,
30 fresh processes: 30/30 correct, p50 3.320 ms, nearest-rank p95 8.145 ms,
maximum 9.038 ms. Shared executor with concurrent workspace testing; timings
are diagnostic, not isolation-controlled performance evidence. No matched Stim
or prior ruqu performance baseline was run. Peak resident memory was not measured.

The existing workspace cannot build with its declared Rust 1.77 toolchain because
its locked zeroize 1.9.0 requires Cargo edition2024 support. A Rust 1.90 workspace
run was started separately; completion is not certified in this receipt.

## Boundaries and replay

No GPU runtime, CUDA transpiler integration, existing QuantumCircuit adapter,
production routing, QPU execution or physical quantum evidence is delivered.
Full compute cost and total review time are unknown. No paid API/QPU job was
submitted. No quantitative safety score is assigned. No optimization generation
or beats-parent performance claim is made.

Run from the repo root:

```sh
cargo +1.77.2 test --manifest-path integrations/independent-stabilizer/Cargo.toml --locked --offline
```

Next milestone: optional circuit adapter and independently validated GPU kernels,
followed by matched Stim/CPU comparisons including transfer and readback cost.
Rollback: revert the additive integration directory and its dedicated workflow.

The adjacent receipt signature uses a fresh ephemeral Ed25519 key. It verifies
artifact integrity only, not an independently trusted issuer or hardware
attestation. Promotion still requires registered provenance and review.
