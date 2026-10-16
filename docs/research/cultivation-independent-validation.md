# Independent cultivation branch validation

The NumPy dense-state executor in `scripts/verify-cultivation.py` independently evaluates the physical trace exported by the native Rust cultivation example. The oracle implements H, T, T-dagger, CNOT, CCZ and I/X/Y/Z fault application using NumPy indexing and complex arithmetic. A separate CSS row-space construction defines the encoded reference states. It does not call ruqu's state-vector or metric routines.

## Reproduction

```bash
cargo build -p ruqu-core --release --example cultivation
python3 scripts/verify-cultivation.py
```

Python 3 and NumPy are required. `--binary`, `--trace` and `--output` accept explicit paths. Without `--trace`, the script exports the current binary's trace; stale intermediate traces are not used. The JSON report records the native binary, trace and oracle SHA256 hashes plus Python and NumPy versions.

## Measured result

29 deterministic branch comparisons passed across the 253-location, 16-qubit circuit. Maximum absolute difference was `5.10702591327572e-15` against a declared tolerance of `2e-11`. This includes:

1. The complete noiseless all-zero branch.
2. X faults at initial, intermediate and final preparation/readout locations.
3. Five Pauli products at the first and final CNOT locations, including Y components.
4. Six Pauli products at the first and final CCZ locations, including YYY and ZZZ.

The measurement covers exact probability of the all-zero branch and three conditional output metrics: full encoded T-state fidelity, codespace probability, and joint codespace/logical-HXY-minus probability. Postselection count until early rejection is also recorded. These are exact selected branches, not finite-shot frequency estimates; confidence intervals are not applicable to the floating-point agreement check.

For the ideal branch, native probability is 1, full encoded-state fidelity is `0.9999999999999993`, codespace probability is `0.9999999999999991`, and joint logical-minus probability is zero. The oracle reaches the same result within the tolerance after 30 postselections.

The final CNOT's X-on-control fault, location 251/code 1, has accepted-branch probability 1 with full encoded-state fidelity and codespace probability both zero. Both implementations agree. This exposes residual detectable physical error: the reported zero joint codespace/logical-minus probability must not be interpreted as a physically error-free output or successful recovery. The tested circuit has no escape or recovery stage.

## Scope and limitations

This check validates independent execution of the same exported circuit and independent computation of its metrics. It does not independently establish correct transcription of the paper, reproduce the Monte Carlo pseudorandom stream, validate physical noise parameters, prove distance three, exhaustively test all fault pairs, or demonstrate a speed advantage. The source trace and CSS checks are shared protocol inputs; a paper-to-circuit transcription error could therefore survive this agreement test.

The reset implementation verifies that reused ancillas have zero excited-state population on these all-zero accepted branches. A protocol requiring nontrivial reset branches will fail explicitly and needs an expanded oracle rather than silently reusing this assumption.

See `cultivation-independent-evidence.json` for every compared branch, numeric metrics and exact hashes. Runtime in that report includes both native subprocess execution and NumPy work; it is validation overhead, not a simulator performance benchmark.

Acceptance: rerun the script from a newly built release binary and obtain 29 finite comparisons within `2e-11`, including the accepted residual-error branch. Any larger mismatch exits nonzero and prevents a new evidence report from being written.
