# ruqu-core

[![Crates.io](https://img.shields.io/crates/v/ruqu-core.svg)](https://crates.io/crates/ruqu-core)
[![Documentation](https://docs.rs/ruqu-core/badge.svg)](https://docs.rs/ruqu-core)
[![License](https://img.shields.io/crates/l/ruqu-core.svg)](https://github.com/ruvnet/ruvector)

**High-performance quantum circuit simulator in pure Rust** — state-vector simulation with SIMD acceleration, noise models, and multi-threading support.

## Features

- **State-Vector Simulation** — Complex128 amplitude representation for exact quantum state evolution
- **Universal Gate Set** — H, X, Y, Z, CNOT, CZ, Toffoli, Rx, Ry, Rz, Phase, SWAP, and custom unitaries
- **Noise Models** — Depolarizing, amplitude damping, phase damping, and custom Kraus operators
- **SIMD Acceleration** — AVX2/NEON vectorized gate application for 2-4x speedup
- **Multi-Threading** — Rayon-based parallelism for large qubit counts
- **Measurement** — Single-qubit, multi-qubit, and partial measurement with state collapse
- **Circuit Optimization** — Gate fusion, cancellation, and commutation rules

## Installation

```bash
cargo add ruqu-core
```

With optional features:

```bash
cargo add ruqu-core --features parallel,simd
```

## Quick Start

```rust
use ruqu_core::{QuantumState, Gate, Circuit, Simulator};

// Create a 3-qubit circuit
let mut circuit = Circuit::new(3);

// Build a GHZ state: |000⟩ + |111⟩
circuit.h(0);           // Hadamard on qubit 0
circuit.cnot(0, 1);     // CNOT: control=0, target=1
circuit.cnot(1, 2);     // CNOT: control=1, target=2

// Execute simulation
let simulator = Simulator::new();
let state = simulator.run(&circuit)?;

// Measure all qubits
let result = state.measure_all();
println!("Measured: {:03b}", result);  // Either 000 or 111
```

## Quantum Gates

| Gate | Description | Matrix |
|------|-------------|--------|
| `H` | Hadamard | Creates superposition |
| `X` | Pauli-X (NOT) | Bit flip |
| `Y` | Pauli-Y | Bit + phase flip |
| `Z` | Pauli-Z | Phase flip |
| `CNOT` | Controlled-NOT | Two-qubit entanglement |
| `CZ` | Controlled-Z | Controlled phase |
| `Rx(θ)` | X-rotation | Rotate around X-axis |
| `Ry(θ)` | Y-rotation | Rotate around Y-axis |
| `Rz(θ)` | Z-rotation | Rotate around Z-axis |
| `SWAP` | Swap qubits | Exchange qubit states |
| `Toffoli` | CCX | Three-qubit AND gate |

## Performance

Benchmarks on Apple M2 (single-threaded):

| Qubits | Gates | Time |
|--------|-------|------|
| 10 | 100 | 0.3ms |
| 15 | 100 | 8ms |
| 20 | 100 | 250ms |
| 25 | 100 | 8s |

With `--features parallel` on 8 cores, 20+ qubits see 3-5x speedup.

## Noise Simulation

```rust
use ruqu_core::noise::{NoiseModel, Depolarizing};

let noise = NoiseModel::new()
    .add_single_qubit(Depolarizing::new(0.01))  // 1% error rate
    .add_two_qubit(Depolarizing::new(0.02));    // 2% for CNOT

let noisy_state = simulator.run_noisy(&circuit, &noise)?;
```

## Related Crates

- [`ruqu-algorithms`](https://crates.io/crates/ruqu-algorithms) — VQE, Grover, QAOA, Surface Code
- [`ruqu-exotic`](https://crates.io/crates/ruqu-exotic) — Quantum-classical hybrid algorithms
- [`ruqu-wasm`](https://crates.io/crates/ruqu-wasm) — WebAssembly bindings

## Architecture

Part of the [RuVector](https://github.com/ruvnet/ruvector) ecosystem. See [ADR-QE-001](https://github.com/ruvnet/ruvector/blob/main/docs/adr/quantum-engine/ADR-QE-001-quantum-engine-core-architecture.md) for design decisions.

## License

MIT OR Apache-2.0
