# Observer world laboratory validation

This document records local acceptance and the repeatable CI gates. The design and scope are in [ADR-002](adr/ADR-002-observer-world-laboratory.md).

## Recorded local evidence, 2026-09-20

Environment: Linux x64, Node v24.19.0, Rust 1.98.1. The unchanged simulator WASM SHA256 is `732acefef718283535272fcc5f6a8d8a599783a2e7e2f0b6be8f3bbb0f42076f`.

| Gate | Observed result |
| :--- | :--- |
| Existing Rust workspace, locked | 1,376 passed, 0 failed, 5 pre-existing ignored doctests |
| Observer Node suites | 40 passed, 0 failed |
| Existing CLI harness smoke suite | 4 passed, 0 failed; kernel fallback identified as `js` |
| TypeScript build | Passed |
| npm production dependency audit | 0 reported vulnerabilities; not a complete security audit |
| Native integration Rust tests | 12 passed, 0 failed |
| Native formatting and strict Clippy | Passed |
| Actual JS to native end-to-end | 4 accepted runs (1, 3, 128, 10,000 events), 14 rejected input cases; no partial output |
| Package dry run | Runtime module and bundled WASM present; no publication performed |

A 100-iteration warmed, alternating local benchmark measured original median 0.099346 ms versus optimized median 0.038407 ms, approximately 2.59x. Original p95 was 0.409491 ms; optimized p95 was 0.100257 ms. Exact distributions matched. The deterministic improvement is 20 to 8 simulator calls, or 60% fewer calls. Timing observations are machine-specific and are not CI thresholds or quantum speedups.

Review discovered and fixed premature CLI process exit truncating large JSON and sparse-array canonicalization. Both have regression tests. CI must rerun these gates on the PR head before merge; local results do not override branch protection.

## What can be established

Software tests can verify the specified quantum model's analytic predictions, deterministic replay, evidence integrity relative to a trusted root, and actual ruField and WorldGraph interoperability. They do not establish a physical Bell experiment, a quantum speedup, alternate realities, a CERN effect, or conscious quantum observers.

The simulator computes both observers' distributions on one classical machine. Its seeded samples are classical pseudorandom numbers. Exact CHSH values are model predictions, not hardware shot estimates with experimental confidence intervals.

## Local acceptance commands

Run from the repository root with Node.js 20 or later and a supported Rust toolchain. The optional integration's first build needs access to its pinned git and Cargo dependencies; later locked builds may use an existing cache.

```sh
node --test examples/observer-lab/lab.test.cjs
node --test cli/tests/observer-lab.test.cjs
node cli/bin/cli.js observer-lab --count 128 --seed 20260920
node cli/bin/cli.js observer-lab --benchmark
cargo test --manifest-path integrations/observer-world/Cargo.toml --locked
cargo fmt --manifest-path integrations/observer-world/Cargo.toml -- --check
cargo clippy --manifest-path integrations/observer-world/Cargo.toml --locked --all-targets -- -D warnings
cargo build --manifest-path integrations/observer-world/Cargo.toml --locked
node integrations/observer-world/verify-e2e.cjs
```

Existing behavior must also be checked:

```sh
npm --prefix cli test
npm --prefix cli run build
cargo test --workspace --locked
```

The npm checks require the repository's development dependencies to be installed through its existing locked workflow. Installation or dependency access failures must be reported as blockers, not silently treated as passing tests.

## Requirement-to-evidence checklist

| Requirement | Necessary evidence | Acceptance |
| :--- | :--- | :--- |
| Q1: real WASM execution | Analytic Bell and CHSH tests, identified WASM bytes | Bell Z probabilities `[0.5, 0, 0, 0.5]`; CHSH `2 * sqrt(2)` within declared tolerance |
| Q2: controls | Enumeration of all 16 deterministic local strategies; dephased run | Local absolute CHSH at most 2; selected dephased value `sqrt(2)` |
| Q3: observer distinction | Recorded and uncomputed observer tests; evidence snapshot before and after recall mutation | Local X probability of zero changes from `0.5` to `1`; evidence unchanged |
| E1: integrity | Altered, reordered, truncated, and wrong-root fixtures | Every fixture rejected; original accepted |
| E2: input boundary | Invalid count, seed, probability, record, size, and memory fixtures | Fail closed without partial success output |
| I1: real integration | Rust tests using pinned upstream types and JSON round trips | Typed events validate; snapshot decodes through real WorldGraph API |
| I2: provenance | Graph inspection and record-to-event mapping tests | No missing source ID or evidence handle; stable replay |
| I3: synthetic status | Source type and output inspection | Simulation never labeled real authenticated sensor acquisition |
| R1: regression | Existing CLI build/tests and Rust workspace tests | No regressions or explicitly approved exceptions |
| P1: optimization | Same correctness tests plus reproducible timing command and context | Exact invariants preserved; no unsupported performance claim |

At default 128 records with an intervention every fourth record, deliberate recall disagreement is 32/128, or 25%. For other record counts or intervention settings, compute the expected fraction from the declared rule rather than assuming it is always 25%.

The current producer bounds the count to 1 through 10,000 and the integer seed to 1 through 4,294,967,295. The command must reject duplicate or unknown flags, noninteger values, and combinations of benchmark mode with run parameters.

The dephased-control optimization reduces state-vector simulation calls from 20 to 8 per evaluation by computing the phase-flipped distribution once per setting rather than once per outcome. The benchmark checks exact output equivalence, warms both paths, alternates measurement order, and reports median and p95 timings together with Node version, platform, architecture, and WASM hash. Timing ratios remain local observations, not guarantees.

## Trusted root boundary

The integration must require the expected ledger root independently of the input document. Reading `ledger.root` from a file and passing it back is useful for testing conversion, but does not independently authenticate that file. To detect malicious replacement, retain the expected root separately through a trusted operational process.

Neither the snapshot nor its hash provides immutable storage, a digital signature, independent timestamping, or ownership authentication. A consistent replacement of both data and trusted root cannot be detected by this design.

## Typed integration checks

Confirm that the Rust manifest pins actual upstream revisions, the geographic dependency has default network features disabled, and Cargo.lock matches the manifest. A build against hand-copied structs or a handwritten "WorldGraph-compatible" object is not sufficient.

Load the emitted snapshot through `wifi_densepose_worldgraph::WorldGraph::from_json`. Inspect `Event`, `SemanticState`, `DerivedFrom`, and any declared support or contradiction relations. All observer beliefs must have nonempty `SemanticProvenance` handles; each graph source endpoint must exist. Synthetic ENU coordinates and derived sequence time must be labeled as such.

WorldGraph does not supply a quantum sensor modality. Do not encode a synthetic event as WiFi CSI, radar, UWB, or presence hardware to make it look physical. This integration uses `DerivedFrom` for evidence lineage, not the physical-support relation `Supports`.

The integration must reject malformed memories even if the original evidence ledger still verifies. Integrity of original evidence does not automatically authenticate separate derived recall fields.

## Review, merge, and rollback

Before merge, inspect the complete diff, confirm no credentials or generated personal data are included, check package contents, and retain the actual commands, test totals, benchmark context, and any exceptions. Do not claim deployment, release, or hardware validation merely because a PR merged.

Rollback uses a reviewed revert. This change must not require reverting a sensor configuration, deleting a live graph, or cancelling an external quantum job because none is created. A future live backend needs separate authorization and acceptance criteria.
