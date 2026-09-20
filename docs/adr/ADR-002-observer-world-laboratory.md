# ADR-002: Observer consistency laboratory with real ruField and WorldGraph contracts

Status: implemented; local acceptance passed. Merge additionally requires CI and repository policy checks in [observer-world-validation.md](../observer-world-validation.md).

Date: 2026-09-20

## Specification

Provide a reproducible research fixture that connects ruQu's bundled quantum simulator to ruField's typed event contracts and WorldGraph's typed belief graph. Its practical value is testing provenance, disagreement, interoperability, and deterministic replay before connecting real sensors or paid quantum hardware.

Actors are a local researcher, a deterministic simulator, two synthetic classical observer records, and an offline integration executable. No human participant data is needed.

Inputs are bounded run parameters and simulator output. The Rust boundary additionally requires an independently supplied trusted ledger root. Outputs are analytic simulation predictions, sampled classical records, deliberately altered memories, typed ruField events, a real WorldGraph snapshot, and explicit provenance. The original evidence and derived beliefs are separate data products.

### Requirements and invariants

| ID | Requirement | Observable acceptance |
| :--- | :--- | :--- |
| Q1 | Execute the actual bundled ruQu WASM engine | Bell probabilities and analytic CHSH value match fixed predictions |
| Q2 | Include a local classical control and a dephased control | Local deterministic bound is 2; the selected dephased CHSH value is sqrt(2) |
| Q3 | Separate a coherent observer register from classical memory | Reversing the register interaction restores the specified interference; changing recall does not change evidence |
| E1 | Detect evidence edits, reordering, and truncation | Verification fails against the separately trusted root |
| E2 | Fail closed on malformed or excessive input | Invalid types, nonfinite values, inconsistent outcomes, and configured size limits are rejected |
| I1 | Use real ruField and WorldGraph Rust types | Pinned dependencies compile and integration tests inspect their output |
| I2 | Preserve provenance through both integrations | Every belief resolves to its source event and evidence handle |
| I3 | Make simulation status explicit | No synthetic event is represented as authenticated physical sensor evidence |
| R1 | Preserve existing behavior | Existing CLI and Rust suites continue to pass |
| P1 | Optimize without altering results | Deterministic and analytic tests pass after optimization; benchmarks report observations rather than universal speedup claims |

The experimental environment is synthetic. Its coordinates and time mapping are not surveyed geography or wall clock measurements. Recall confidence is not a calibrated physical probability. Event support and contradiction refer to encoded records, not proof about external reality.

### Explicit exclusions

There is no quantum hardware execution, proof of physical nonlocality, quantum computational advantage, conscious observer, alternate universe access, CERN connection, or explanation of the Mandela effect. There is no production sensor admission, autonomous actuation, enforcement system, RuVector database ingestion, hosted deployment, or npm publication implicit in this change.

The word "proof" means executable verification of these software contracts and agreement with established analytic predictions. It does not mean a proof of new physics or a formally verified implementation.

## Pseudocode

```text
read bounded run parameters
reject parameters outside the documented contract
load bundled WASM simulator and identify its bytes
compute fixed Bell, classical, dephased, and reversible-observer predictions
generate reproducible classical samples from a seeded sampler
append evidence records to a hash chain
derive observer memories without mutating evidence
emit run, provenance, evidence root, and simulation limitations

read bounded integration input and independently supplied trusted root
validate schema, evidence records, chain, and root before emitting output
validate derived memories against the recorded intervention policy
map accepted evidence into ruField synthetic event types
construct a WorldGraph room, source events, and observer semantic states
attach evidence provenance and native derived_from/contradicts relations
serialize real typed events and WorldGraph snapshot
on any invalid input: return failure; do not emit a partial success document
```

Success walk: an unchanged deterministic run verifies, its synthetic events retain source handles, and its observer disagreement reflects only the declared intervention.

Failure walk: a single source outcome is changed. A chain check rejects it before integration output. Supplying a new self-consistent chain also fails against the original trusted root. Replacing both the data and the independently trusted root is outside what hashing can detect.

Replay is deterministic for a fixed seed, count, intervention policy, and simulator binary. The integration creates an offline snapshot, not an automatically retried mutation of a live graph. Repeating conversion must not append duplicate events to a remote service because no remote writes occur.

## Architecture

| Component | Ownership and boundary |
| :--- | :--- |
| `cli/src/observer-lab.cjs` | Production package location of the laboratory engine; consumes the bundled WASM without a new npm runtime dependency |
| `cli/bin/cli.js` | Exposes `observer-lab` using the existing CLI command surface |
| `examples/observer-lab/` | Runnable example and deterministic acceptance tests, delegating computation to the CLI implementation |
| `integrations/observer-world/` | Standalone optional Rust package; validates input, then constructs real ruField and WorldGraph values |
| Original ledger | Authoritative evidence only relative to a separately trusted root; never reconstructed from observer belief text |
| Derived field events and graph | Disposable, reproducible projections; not an append-only evidence store |

The optional Rust integration is separate from the main Cargo workspace. Users of the existing simulator do not acquire the spatial integration's dependency graph or build cost. The standard CLI remains a local simulation; the Rust tool is explicitly invoked when typed ecosystem integration is needed.

### Dependency provenance

| Repository | Pinned revision | Relevant actual crates |
| :--- | :--- | :--- |
| `ruvnet/rufield` | `7179a2efc706993ee0d87f0093e6a0e9e3dc5017` | ruField typed event crates selected by the integration manifest |
| `ruvnet/worldgraph` | `9b1c79c836cdacfb7b44f058c593157bac4c1dab` | `wifi-densepose-worldgraph`, `wifi-densepose-geo` |

The geographic dependency must disable default network features. Exact git revisions and the integration lockfile are the reproducibility boundary. A pinned revision is not a security audit of every upstream dependency.

WorldGraph's actual model has `Event` and `SemanticState` variants, not invented `Agent` or `QuantumSensor` nodes. `SemanticProvenance` carries evidence handles, model version, calibration version, and privacy decision. The integration uses `LocatedIn`, `DerivedFrom` and `Contradicts`. WorldGraph schema version 2 stores stable edge records with explicit `id`, `from`, `to`, and `edge` fields.

WorldGraph documents `Supports` primarily for physical or clock support between sensor or RF link nodes. The integration deliberately does not use it for evidence lineage; `DerivedFrom` expresses that relationship without a semantic extension. No fake physical sensor modality is introduced.

WorldGraph's helper can skip an unknown evidence source. The integration therefore owns source existence checks. Hash verification, probability ranges, count limits, and content provenance are also boundary responsibilities, not assumptions about downstream serde types.

### Alternatives

| Design | Delivery and runtime tradeoff | Accuracy and security tradeoff | Decision |
| :--- | :--- | :--- | :--- |
| Handwritten JSON resembling both projects | Lowest build cost; no real dependency execution | Easy schema drift; not an actual integration | Rejected |
| Add all ecosystem crates to the main workspace | One build graph; larger dependency and compile surface for every user | Tight coupling and broader regression scope | Rejected |
| Existing WASM CLI plus optional pinned Rust bridge | Separate opt-in compile cost; linear projection of records | Real typed contracts, narrow trust boundary, reproducible dependencies | Selected |
| Real QPU or live sensor backend | Provider latency, cost, calibration, and authorization requirements | Different experiment and substantially larger threat model | Deferred |

No hardware throughput or latency is estimated from a local two-qubit simulator. Simulation work is fixed for the configured experiments; ledger generation and projection scale with record count. Benchmarks must separate compute time, JSON serialization, dependency compilation, and external hardware costs.

## Security and data lifecycle

Treat integration input as untrusted. Require bounded input and a separately supplied trusted root, validate all records, and perform conversion only after verification. Do not load credentials, contact providers, execute payloads, or accept external commands through input fields.

The hash chain detects alteration only relative to a trusted root. It supplies neither identity signatures nor external time anchoring. Exported classical records cannot preserve or restore quantum coherence. Generated recall records are not scientific evidence of changed history.

Synthetic data may be recreated and discarded. A real sensor or QPU extension would require a new ADR covering authenticated acquisition, calibration, retention, consent where relevant, uncertainty, and provider spend authorization.

## Refinement and completion gates

Implement the smallest independently testable units: simulator engine, verification boundary, actual typed integrations, then command and package validation. An optimization is accepted only when existing analytic and replay invariants remain unchanged. Record measured benchmark inputs and environment; do not turn one local timing into a general performance claim.

The validation document is the requirement-to-evidence checklist. The pull request must retain concrete command outcomes and explicitly identify any unrun checks. Neither a draft checklist nor a documentation status counts as a passing test.

## Rollout and rollback

Roll out as an additive CLI command and opt-in offline Rust package. Review the exact merge commit and required CI checks before merging. Do not bypass protected branch checks, publish packages, or enable production integrations as a side effect.

Rollback is a normal reviewed revert of this feature's commit or merge. Existing simulator commands remain the fallback. There is no production database migration, external job, or sensor setting to reverse. A future publishing change needs its own versioning and rollback decision.

Residual risk owner: the maintainer accepting the integration. Main remaining risks are semantic overinterpretation, future upstream schema changes, and treating a caller-supplied root as automatically trusted. Mitigations are explicit simulation labels, pinned contracts, fail-closed verification, and review of any physical-world extension.
