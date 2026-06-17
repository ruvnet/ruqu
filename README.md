# ruqu

Quantum computing in pure Rust + WebAssembly:
- **Quantum circuit simulator** — `ruqu-core` (state-vector sim, SIMD, noise models), `ruqu-algorithms` (QAOA, etc.), `ruqu-exotic`, `ruqu-wasm`.
- **`ruqu`** (`crates/ruQu`) — a classical "nervous system for quantum machines": real-time coherence assessment via dynamic min-cut.

> Extracted from [ruvnet/ruvector](https://github.com/ruvnet/ruvector) per ADR-257.
> Builds standalone: `cargo build`. Optional `ruvector-mincut`-backed features in
> the `ruqu` crate pull from crates.io. npm: `@ruvector/ruqu-wasm` (`npm/packages/ruqu-wasm`).

## License
MIT © Ruvector Team
