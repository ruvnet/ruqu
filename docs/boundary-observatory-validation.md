# Boundary Observatory validation

Verified locally on 2026-09-25. These are synthetic model and software results,
not physical measurements or a new discovery.

## Reproduce

```sh
node examples/boundary-lab/run.cjs
node examples/boundary-lab/package-smoke.cjs
node --test cli/tests/boundary-lab.test.cjs
python examples/boundary-lab/reference.py
```

The [saved evidence summary](../examples/boundary-lab/evidence.json) contains the
parameters, metrics, separate process comparison, replay result and export counts.
Use the example's `--output NEW_DIRECTORY` option for the complete report.
The summary alone cannot substitute for the full report during verification.

Protocol and source commitment:

`af87999640efb817dff4b57f9f4187f715a6e9955afe2a44998d7f9cca2f9dc7`

Full report root:

`b1a4da50b29f38c7bdaf9ec51b90c058011825605ebd70fa8309fa3c9cf36367`

These match the original implementation before workspace maintenance removed the
unpublished checkout. Production modules were reconstructed from the implementation
record and freshly validated against main. Exact root reproduction binds both
their source fingerprints and the complete result. This PR excludes the separate
unpublished quantum hypothesis laboratory and its ten tests.

## Default experiment

Seed 20260925. Four arms of 100 trials each, with 199 signal emitters total.

1. Exact scene success: 90 of 100, including frequency set, location and quadrant.
2. Marginal descriptive Wilson interval: 82.56 to 94.48 percent.
3. Emitters localized within 0.25 metres: 199 of 199, with zero misses.
4. Correct quadrants: 189 of 199. Quadrant boundaries account for failed scenes.
5. Median location error: 0.0324453 metres versus centroid baseline 0.0794957 metres.
6. Median error reduction: 59.19 percent.
7. Empty scenes: zero detections in 100.
8. Single sensor injections: all 100 rejected.
9. Deliberate model mismatch scenes: all 100 rejected.
10. Each control's marginal descriptive false acceptance interval: 0 to 3.70 percent.
11. Export: 1200 field events, 608 graph nodes, 806 graph edges.

Mismatch rejection means declining a location assertion, not proving the absence
of an emitter. Exports are local ruqu contracts, not native service integrations.

## Checks

1. `npm test` in `cli`: 4 Vitest tests and 40 Node tests pass, including 13 boundary tests.
2. Existing observer example suite: 13 tests pass. Combined JavaScript total: 57.
3. TypeScript build passes.
4. Public CLI example passes plan, run, separate predict, verify and project.
5. Extracted package runs and verifies without installing dependencies; nothing published.
6. All 299 detectable channels agree with the independent NumPy grid calculation.
7. Maximum residual discrepancy: 3.552713678800501e-15 dB.
8. Maximum grid location separation from the SciPy continuous optimum: 0.0355055 metres.
9. Maximum admitted 2000 trial run replays and fits the 16 MiB input limit.
10. Tampered inputs, source fingerprints, labels, verdicts and rehashed forgeries fail verification.
11. Malformed measurements, physical evidence, label leakage, oversized files and FIFOs fail closed.
12. Entirely missing measurements produce a negative verdict and count every source as missed.

Native Rust tests were not run locally because Cargo is unavailable. This PR
changes no Rust or WASM code. The existing Rust CI job remains a merge gate.
Exact replay assumes compatible JavaScript arithmetic; independent numerical
checks use tolerances instead of claiming universal bitwise runtime equivalence.

## Security and remaining gates

Read only scans with Ruflo CLI 3.25.6 cover all four new production modules.
`npm audit --omit=dev --audit-level=high` returned zero vulnerabilities on
2026-09-25. The advisory service supplied no feed snapshot timestamp. Production
dependencies and their lockfile are unchanged. Manual review covers bounded local
reads, provenance, raw lineage, strict synthetic input and no production network
or subprocess calls. Scanning cannot establish scientific validity.

The simulator and estimator share propagation assumptions. Hardware calibration,
real indoor multipath, cochannel source separation, authentic acquisition and
native adapters remain future maintainer gates. The next acceptance test is a
separate frozen physical protocol with independent labels and calibration, using
unchanged thresholds on held out sensor captures. No quantum advantage, QMM,
holography or unexplained physics is established here.
