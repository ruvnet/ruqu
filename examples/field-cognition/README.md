# Field cognition experiment 1

Experimental comparison using actual RuQu WASM and a local bounded field reference model. The candidate lost to the model informed classical Bayesian baseline. No consciousness, Phi, quantum hardware advantage or production readiness is claimed.

## Reproduce

Requires Node 20 or later and npm. From this directory:

```sh
npm ci --ignore-scripts --no-audit --no-fund
npm test
npm run benchmark
node verify.cjs
```

The verifier checks pinned source/protocol hashes and raw output against the ruOS reference. Latencies and RSS vary by machine. Fresh results overwrite results.json and raw-results.jsonl; use a new checkout when preserving historical outputs.

See ADR-001.md for the equations, frozen protocol, negative results, limits and next experiment. RuQu 0.2.0 is pinned by package-lock.json. The repository source reference is eb127994aaf8cdb074ebcc3ff32c1a798b79efa8, but the packaged WASM is bound by its npm integrity and file hash; no reproducible source-to-package build is asserted.

The Bayesian comparator is explicitly informed of generator noise and hazard. This experiment tests a matched synthetic model, not superiority across real tasks. No learning parameters were fitted. Five seeds are now disclosed and cannot be called unseen data in future tuning.
