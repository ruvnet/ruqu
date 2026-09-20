# Observer consistency laboratory

Runnable research prototype using the actual bundled ruQu WASM engine. Requires Node.js 20 or later. No installation, network, credentials, or paid quantum jobs needed.

From the repository root:

```sh
node --test examples/observer-lab/lab.test.cjs
node examples/observer-lab/lab.cjs
```

## Fixed predictions

* Ideal Bell state: CHSH S = 2 sqrt(2).
* All 16 deterministic local hidden-variable strategies: absolute S <= 2.
* Equal mixture of Bell states with opposite phase: S = sqrt(2) at the same settings.
* Copy system's computational-basis record into a coherent observer qubit: system X-basis probability of zero = 1/2.
* Reverse that CNOT before readout: probability = 1.
* Alter one quarter of Bob's classical memories: disagreement = 25%, original evidence unchanged.

These are established quantum-model predictions, not new physics hypotheses. The simulated Bell violation does not demonstrate physical nonlocality: a classical computer calculates both observers' outcomes jointly. It does not test Bell loopholes. A coherent observer qubit is not a conscious observer. Uncomputation is not deletion of an already exported classical measurement record.

## Method and boundaries

The engine computes exact probabilities without measurement gates. A documented seeded classical xorshift32 sampler generates 128 joint Z-basis outcomes. Sampling is not quantum randomness and is not cryptographically secure. CHSH is computed from exact probabilities, not finite hardware shots; therefore no experimental confidence interval is claimed. Dephasing uses an explicit equal classical mixture of two phase-related state-vector runs.

Evidence and altered memories are separate arrays. SHA256 links evidence records; verification requires a separately trusted terminal hash. This detects edits and truncation relative to that root, not malicious replacement of the log AND root. No append-only storage, external timestamp, signature, RuVector ingestion, hardware backend, geospatial engine, or agent intelligence is claimed.

Outputs identify the protocol and actual WASM binary hash. Each evidence record binds that hash. The bundled simulator binary is used directly; it is not rebuilt by the Node command. The original example delegates to the production package module in `cli/src/observer-lab.cjs`.

The [native integration](../../integrations/observer-world/README.md) constructs actual ruField synthetic events and WorldGraph beliefs after validation. This is an optional compiled projection, not live sensor or database ingestion. See [acceptance checks](../../docs/observer-world-validation.md).

## Next integration contract

Store original evidence in an authoritative append-only store with an externally anchored root. Index copies in RuVector with run ID, observer ID, source hash, setting and intervention tags. Semantic retrieval must never rewrite evidence. Add a QPU adapter only with explicit provider/job identifiers, shots, calibration data, measurement settings and uncertainty analysis. Compare against classical controls; no quantum advantage follows from this prototype.

## Acceptance

All tests must pass, including analytic predictions, no-signalling marginals, seeded replay, corruption detection and rejected truncation. This is software verification only, not evidence about CERN, Mandela effects or alternate universes.
