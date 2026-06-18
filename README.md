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
- **Coherence-aware** — a classical "nervous system for quantum machines" that assesses qubit
  coherence in real time via dynamic min-cut.

## Crates

| Crate | What it does |
|-------|--------------|
| [`ruqu-core`](crates/ruqu-core) | High-performance state-vector **quantum circuit simulator** — SIMD acceleration, noise models, multi-threading. |
| [`ruqu-algorithms`](crates/ruqu-algorithms) | Production **quantum algorithms** — **VQE** for chemistry, **Grover's** search, **QAOA** optimization, **Surface Code** error correction. |
| [`ruqu-exotic`](crates/ruqu-exotic) | Experimental **quantum–classical hybrid** algorithms — quantum memory decay, interference search, reasoning error correction, swarm interference for AI systems. |
| [`ruqu-wasm`](crates/ruqu-wasm) | **WebAssembly** bindings — run quantum simulations in the browser (25-qubit, VQE/Grover/QAOA). |
| [`ruqu`](crates/ruQu) | Classical **coherence engine** — real-time coherence assessment for quantum machines via **dynamic min-cut**. |
| [`ruqu-possibility`](crates/ruqu-possibility) | **Structural possibility runtime** — possibility fields, interference scoring, coherence gating, and auditable collapse receipts (ADR-258). |
| [`ruqu-rag`](crates/ruqu-rag) | **Interference reranking** — possibility-field retrieval that suppresses contradicted candidates and emits collapse receipts (ADR-258). |
| [`ruqu-agent`](crates/ruqu-agent) | **Swarm collapse consensus** — interference-based multi-agent consensus with reasoning error correction (ADR-258). |
| [`ruqu-sensing`](crates/ruqu-sensing) | **Structural anomaly detection** — telemetry → syndrome streams → fault localization (ADR-258). |
| [`ruqu-receipts`](crates/ruqu-receipts) | **Governance evidence** — tamper-evident, replayable collapse/audit logs (ADR-258). |

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

## Structural Possibility Runtime (ADR-258)

Beyond simulation, ruqu doubles as a **structural possibility runtime for AI
systems** — a layer that holds multiple plausible states, amplifies coherent
evidence, suppresses contradictory paths, gates risky actions, and emits
**collapse receipts**. See
[ADR-258](docs/adr/ADR-258-structural-possibility-runtime.md).

```rust
use ruqu_possibility::{Possibility, PossibilityField, CoherenceGate};

let field = PossibilityField::new(vec![
    Possibility::new("strong",      "well-cited answer",        0.9, 0.0),
    Possibility::new("contradicted","plausible but refuted", 0.6, std::f64::consts::PI),
]);

let decision = CoherenceGate::with_defaults().evaluate(&field); // PERMIT / DEFER / DENY
let (selected, receipt) = field.collapse(42).unwrap();          // deterministic + auditable
```

Try the interference-vs-cosine retrieval demo:

```bash
cargo run -p ruqu-rag --bin quantum_rag_demo
```

## Management console (web)

A static, browser-based **management console** for the structural possibility
runtime (ADR-258) lives in [`web/`](web). It is a plain ES-module web app driven
by a WASM module, with a live **Sensing / Live Gate** panel that consumes a
WebSocket telemetry stream. See [`web/README.md`](web/README.md) and
[ADR-258](docs/adr/ADR-258-structural-possibility-runtime.md).

```bash
# build the WASM bundle, then serve the static app
wasm-pack build crates/ruqu-console-wasm --target web --out-dir web/pkg --out-name ruqu_console
python3 -m http.server 8099 --directory web   # open http://localhost:8099/
```

It deploys to **GitHub Pages** via
[`.github/workflows/pages.yml`](.github/workflows/pages.yml) (repo Settings →
Pages → Source: GitHub Actions). Pages is static-only, so the Sensing panel uses
an external `ws://`/`wss://` endpoint or the built-in simulated feed — see
[`web/server-example/`](web/server-example) for an example WebSocket server.

## Use cases

Quantum algorithm research · variational quantum eigensolver (VQE) for quantum chemistry ·
combinatorial optimization with QAOA · quantum error-correction (surface codes) experiments ·
NISQ noise studies · teaching quantum computing · browser-based quantum demos.

## License

MIT © Ruvector Team. Part of the [ruvector](https://github.com/ruvnet/ruvector) ecosystem
(extracted per ADR-257).
