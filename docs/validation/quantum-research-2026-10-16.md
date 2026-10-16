# Quantum research implementation acceptance

Date: 16 October 2026. Base: `eb127994aaf8cdb074ebcc3ff32c1a798b79efa8`.

These measurements are local classical software controls. They are not physical quantum processor measurements. The JSON evidence includes source hashes, runtime versions, assumptions and reproducible commands.

| Requirement | Implementation | Evidence |
| :--- | :--- | :--- |
| Exact factory reference | Integer syndrome/weight dynamic program with cached spectrum | Exhaustive 32,768 pattern reference, independent closed formula and dense error-register projector; 10 Rust factory tests |
| Explicit retry and transfer costs | Recursive input consumption and expected qubit rounds; transfer selection can amplify upstream error | Adversarial selection, zero yield, overflow and positive-error underflow tests |
| Reproducible CLI | Node `magic-factory`; strict arguments and complete JSON | 9 new Node tests; 6 probability settings match Rust probabilities, seeded counts and resource estimates |
| Sparse observable simulation | Bounded Heisenberg Pauli propagation with T branching, stochastic Pauli noise and postselection | 64 circuits times 5 observables match an independent dense simulator; 7 integration tests, including independently reproduced underflow and work-budget regressions |
| Large structured register | 128-qubit encoded T observable through 127 CNOT gates | One Pauli term; expectation `0.7071067811865476`; no state-vector allocation |
| Correlated detector faults | Bounded exact individual-error search on detector hypergraphs | 7 tests, including 512 model/syndrome comparisons against exhaustive enumeration, budget exhaustion and logical coset distinction |
| Animated documentation | Three native SVGs with static content and reduced motion | XML, accessibility, local links and external resource isolation checks; rendered visual inspection |
| Existing behavior | Workspace, CLI and bundled WASM commands | Full workspace: 1,398 passed, 0 failed, 5 existing ignored; latest Pauli suite rerun after boundary fixes: 7 passed; CLI: 4 smoke tests and 36 Node tests passed |
| Distribution | Packed npm artifact from this checkout | 21 files; extracted `magic-factory` output matches the source command; package not published |
| Static and dependency checks | Strict Clippy, targeted rustfmt, TypeScript build and npm production audit | All pass; npm audit reports zero production vulnerabilities; Rust advisory scan was not run and neither dependency lockfile changed |

The full workspace run compiled the Pauli suite before the final two boundary regressions were added. The final Pauli implementation and all seven tests were rerun afterward, together with strict Clippy, the Rust/Node acceptance script and formatting checks. CI executes the complete current source tree.

## Reproduce

```bash
cargo test --workspace --locked
cargo clippy -p ruqu-core --locked --all-targets -- -D warnings
npm ci --ignore-scripts --prefix cli
npm test --prefix cli
npm run build --prefix cli
npm audit --prefix cli --omit=dev --audit-level=high
node scripts/verify-research.cjs
python3 scripts/verify-docs.py
cargo run --release -p ruqu-core --locked --example magic_states -- --benchmark
```

At input error 0.01 the ideal model returns acceptance `0.8600903336704236` and conditional output error `0.0000360876839653233`. SplitMix64 seed 42 over 100,000 attempts produces 86,063 accepted attempts, 13,937 rejects and 3 accepted logical errors. Zero observed errors never imply zero uncertainty.

## Performance measurement

The optimized release run over 500 cold computations measured 612,129 ns for the exact dynamic program and 56,928,570 ns for exhaustive enumeration, about 93 times less elapsed setup time on this host. Cached lookup measured 7,341 ns over 500 calls. Matching integer spectra were checked. This is a spectrum setup microbenchmark, not an end-to-end quantum simulation speedup or a portable guarantee. Raw values are in [the release benchmark](weight-spectrum-release-2026-10-16.json).

## Security and scientific boundaries

The new model paths introduce no dependencies, providers, credentials, subprocess execution, network access or hardware control. The development acceptance script runs fixed Cargo/Node entrypoints locally. Untrusted CLI numbers face finite range checks, simulation shot limits and round limits. The native decoder has a mandatory search node budget. Sparse propagation has qubit, term, storage and work limits and rejects unsupported gates and numerically unresolved nonzero branches.

Independent review found and verified fixes for coefficient underflow appearing as impossible selection and uncharged final Pauli reductions. Exact impossible branches still return an absent conditional expectation. A preexisting obsolete Clippy lint entry was removed so strict linting works with the available compiler.

Remaining limits: ideal distillation does not model faulty Clifford operations, correlated inputs or leakage. Pauli noise is not a general coherent hardware noise model. Hypergraph search has exponential worst case cost and optimizes individual errors rather than logical coset probabilities. Full Willow cultivation, the complete Surti and Clifford-stabilizer simulation algorithms, unfolded physical circuits and upstream Tesseract are research directions, not completed ports.

Cargo lockfile SHA256: `22934c537eb7ce8f003d8c146df50cb01ee956910350f4256edf81fa95507efa`.
CLI lockfile SHA256: `436b612f5b0a352ca9045f7039a451a565a54a360c72d2e361a73b224fd8f3cf`.

Rollback: revert the isolated implementation commit; no database migration, service deployment or package publication is involved.
