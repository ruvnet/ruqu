# ruqu — Quantum Computing in Pure Rust + WebAssembly

[![Rust](https://img.shields.io/badge/Rust-2021-orange?logo=rust)](https://www.rust-lang.org)
[![WebAssembly](https://img.shields.io/badge/WebAssembly-ready-654ff0?logo=webassembly&logoColor=white)](#webassembly)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](#license)

**`ruqu`** is a fast, dependency-light **quantum computing toolkit written in pure Rust** — a
state-vector **quantum circuit simulator** with SIMD acceleration, realistic **noise models**,
and multi-threading, plus production **quantum algorithms** (VQE, Grover, QAOA, surface-code
error correction) and a novel **real-time coherence engine**. It runs natively *and* in the
browser via **WebAssembly**.

> Quantum circuit simulation, VQE quantum chemistry, QAOA optimization, Grover search, surface-code
> QEC, and 25-qubit in-browser WASM — no Python, no C++, no heavyweight dependencies.

## Why ruqu

- **Pure Rust, no FFI** — portable, memory-safe, easy to embed; compiles to native and `wasm32`.
- **SIMD + multi-threaded state-vector engine** — high-throughput circuit simulation.
- **Realistic noise models** — depolarizing, dephasing, amplitude damping for NISQ-style studies.
- **Batteries-included algorithms** — VQE (chemistry), Grover's search, QAOA (combinatorial
  optimization), and Surface Code error correction, ready to use.
- **Runs in the browser** — `ruqu-wasm` exposes circuits, VQE, Grover and QAOA to JavaScript with
  ~25-qubit support.
- **Coherence-aware** — a classical "nervous system for quantum machines": a real-time
  *structural-health gate* that assesses qubit coherence via dynamic min-cut and decides whether
  it's safe to act. It is a health gate, not a surface-code syndrome decoder.

## Crates

| Crate | What it does |
|-------|--------------|
| [`ruqu-core`](crates/ruqu-core) | High-performance state-vector **quantum circuit simulator** — SIMD acceleration, noise models, multi-threading. |
| [`ruqu-algorithms`](crates/ruqu-algorithms) | Production **quantum algorithms** — **VQE** for chemistry, **Grover's** search, **QAOA** optimization, **Surface Code** error correction. |
| [`ruqu-exotic`](crates/ruqu-exotic) | Experimental **quantum–classical hybrid** algorithms — quantum memory decay, interference search, reasoning error correction, swarm interference for AI systems. |
| [`ruqu-wasm`](crates/ruqu-wasm) | **WebAssembly** bindings — run quantum simulations in the browser (25-qubit, VQE/Grover/QAOA). |
| [`ruqu`](crates/ruQu) | Classical **coherence engine** — real-time coherence assessment for quantum machines via **dynamic min-cut**. |
| [`ruqu-possibility`](crates/ruqu-possibility) | **Structural possibility runtime** — possibility fields, interference scoring, coherence gating, and auditable collapse receipts. |
| [`ruqu-rag`](crates/ruqu-rag) | **Interference reranking** — possibility-field retrieval that suppresses contradicted candidates and emits collapse receipts. |
| [`ruqu-agent`](crates/ruqu-agent) | **Swarm collapse consensus** — interference-based multi-agent consensus with reasoning error correction. |
| [`ruqu-sensing`](crates/ruqu-sensing) | **Structural anomaly detection** — telemetry → syndrome streams → fault localization. |
| [`ruqu-receipts`](crates/ruqu-receipts) | **Governance evidence** — tamper-evident, replayable collapse/audit logs. |

## Install

```toml
# Cargo.toml
[dependencies]
ruqu-core = "2.2"
ruqu-algorithms = "2.2"   # VQE, Grover, QAOA, surface-code QEC
```

```bash
cargo add ruqu-core ruqu-algorithms
```

## Quick start

```rust
use ruqu_algorithms::qaoa::{run_qaoa, Graph, QaoaConfig};

// Solve a MaxCut instance with QAOA on the ruqu state-vector simulator.
let graph = Graph::from_edges(4, &[(0, 1), (1, 2), (2, 3), (3, 0)]);
let result = run_qaoa(&graph, &QaoaConfig::default());
println!("best cut = {:?}", result.best_bitstring);
```

## WebAssembly

```bash
# build the browser bundle
wasm-pack build crates/ruqu-wasm --target web
```

```js
import init, { simulate } from "./pkg/ruqu_wasm.js";
await init();
// run a quantum circuit entirely in the browser — up to ~25 qubits
```

## Build

```bash
cargo build --release                            # native
cargo test                                       # run the test suite
wasm-pack build crates/ruqu-wasm --target web    # WASM
```

## Command-line quantum (npx)

Run quantum circuits from your terminal via **[`@ruvector/ruqu`](https://www.npmjs.com/package/@ruvector/ruqu)** —
the `ruqu-wasm` state-vector simulator compiled to WebAssembly, wrapped in a metaharness agent CLI:

```bash
npx @ruvector/ruqu simulate --qubits 4    # GHZ state-vector simulation
npx @ruvector/ruqu grover --qubits 3 --target 5
npx @ruvector/ruqu qaoa --nodes 4         # QAOA MaxCut on a ring
npx @ruvector/ruqu capabilities           # gates, algorithms, limits
npx @ruvector/ruqu doctor                 # verify the quantum WASM
```

Sources in [`cli/`](cli); the bundled `--target nodejs` WASM runs up to 25 qubits in Node — no native addon.

## Structural possibility runtime

Beyond simulation, ruqu doubles as a **governed decision layer for AI systems**.
Its first job is accountability:

- **Auditable decision receipts.** Every decision is written to a tamper-evident,
  hash-chained log — *what* was chosen, *why*, and *what was rejected* — so any
  decision can be replayed and verified after the fact.
- **A DEFER state for human oversight.** Beyond a simple yes/no, the runtime can
  step back and hand off — declining to act on its own when confidence is too low
  and routing the case to a person.
- **PERMIT / DEFER / DENY risk gating.** High-impact actions pass a gate that
  permits the safe ones, defers the uncertain ones for review, and denies the
  unsafe ones outright.

Underneath that, instead of committing to the single highest-scoring answer, it
keeps several plausible options "in play" at once and lets supporting evidence
reinforce while contradictory evidence cancels out — then collapses to a choice,
gates it, and records the receipt.

It's useful wherever you'd otherwise pick a top result and hope it's right:
retrieval that resists confidently-wrong sources, multi-agent decisions that
don't silently act on weak consensus, and telemetry monitoring that flags
correlated failures.

### Prior art & positioning

Using interference — letting evidence reinforce or cancel rather than just adding
up scores — to rank and decide is not new: it has a substantial research lineage
spanning information retrieval, quantum-inspired cognition and decision models,
and complex-valued matching. ruqu does not claim to invent that principle. Its
contribution is the **integrated, auditable runtime**: combining interference
scoring with a coherence/entropy gate and tamper-evident, hash-chained collapse
receipts, mapped to concrete governance practice (auditable logging and a human
in the loop). For the full landscape, baselines, and citations, see
[`docs/research/sota-landscape.md`](docs/research/sota-landscape.md).

```rust
use ruqu_possibility::{Possibility, PossibilityField, CoherenceGate};

let field = PossibilityField::new(vec![
    Possibility::new("strong",      "well-cited answer",     0.9, 0.0),
    Possibility::new("contradicted","plausible but refuted", 0.6, std::f64::consts::PI),
]);

let decision = CoherenceGate::with_defaults().evaluate(&field); // PERMIT / DEFER / DENY
let (selected, receipt) = field.collapse(42).unwrap();          // deterministic + auditable
```

See it pick the well-supported answer over a contradicted-but-similar one:

```bash
cargo run -p ruqu-rag --bin quantum_rag_demo
```

## Management console (web)

A browser-based **dashboard** for the runtime lives in [`web/`](web): explore
possibility fields, compare ordinary vs. interference-based search, run
multi-agent consensus, watch a **live coherence gauge** fed by a real-time
telemetry stream, and inspect/verify the decision receipts. It's a plain web app
(no build tooling to install) running the runtime directly in the browser via
WebAssembly. Full details in [`web/README.md`](web/README.md).

```bash
# build the WebAssembly bundle, then serve the app locally
wasm-pack build crates/ruqu-console-wasm --target web --out-dir web/pkg --out-name ruqu_console
python3 -m http.server 8099 --directory web   # open http://localhost:8099/
```

It publishes to **GitHub Pages** automatically via
[`.github/workflows/pages.yml`](.github/workflows/pages.yml) (enable it under repo
Settings → Pages → Source: GitHub Actions). Because GitHub Pages only serves
static files, the live panel either connects to a WebSocket server you run
yourself (an example is in [`web/server-example/`](web/server-example)) or falls
back to a built-in simulated feed, so the dashboard works with no backend at
all.

## Use cases

Quantum algorithm research · variational quantum eigensolver (VQE) for quantum chemistry ·
combinatorial optimization with QAOA · quantum error-correction (surface codes) experiments ·
NISQ noise studies · teaching quantum computing · browser-based quantum demos.

## License

MIT © Ruvector Team. Part of the [ruvector](https://github.com/ruvnet/ruvector) ecosystem.
