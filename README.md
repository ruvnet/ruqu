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

## Agent CLI

An agent-harness CLI ships on npm as **[`@ruvector/ruqu`](https://www.npmjs.com/package/@ruvector/ruqu)** —
boots the [metaharness](https://github.com/ruvnet/agent-harness-generator) kernel + a Claude Code
host adapter with a self-evolving agent loop:

```bash
npx @ruvector/ruqu init     # boot the kernel + host adapter
npx @ruvector/ruqu doctor   # verify the install
```

(Sources in [`cli/`](cli). The kernel resolves native → wasm → js; the published beta currently
runs the `js` backend.)

## Use cases

Quantum algorithm research · variational quantum eigensolver (VQE) for quantum chemistry ·
combinatorial optimization with QAOA · quantum error-correction (surface codes) experiments ·
NISQ noise studies · teaching quantum computing · browser-based quantum demos.

## License

MIT © Ruvector Team. Part of the [ruvector](https://github.com/ruvnet/ruvector) ecosystem
(extracted per ADR-257).
