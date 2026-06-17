# @ruvector/ruqu

**Quantum computing from your terminal** — a state-vector quantum circuit simulator compiled to
**WebAssembly** (the pure-Rust [`ruqu`](https://github.com/ruvnet/ruqu) crates), wrapped in a
[metaharness](https://github.com/ruvnet/agent-harness-generator) agent CLI.

```bash
npx @ruvector/ruqu capabilities          # what it can do
npx @ruvector/ruqu simulate --qubits 4   # GHZ state-vector simulation
npx @ruvector/ruqu grover --qubits 3 --target 5
npx @ruvector/ruqu qaoa --nodes 4        # QAOA MaxCut on a ring
npx @ruvector/ruqu doctor                # verify kernel + quantum WASM
```

## Commands

| Command | What it does |
|---|---|
| `simulate [--qubits N]` | Run a GHZ/Bell circuit on the WASM state-vector simulator (up to 25 qubits). |
| `grover [--qubits N --target T --seed S]` | Grover amplitude amplification / search. |
| `qaoa [--nodes N --p P]` | QAOA MaxCut on a ring graph. |
| `capabilities` | List gates, algorithms, qubit/memory limits. |
| `init` · `doctor` | Boot / verify the agent-harness kernel **and** the quantum WASM. |
| `version` | Kernel + WASM versions. |

Gates: `h x y z s t rx ry rz cnot cz swap rzz measure reset barrier`. Up to **25 qubits**.

## How it works

The CLI bundles a `--target nodejs` WebAssembly build of the `ruqu-wasm` crate (real state-vector
simulation in Rust → WASM) and loads it directly in Node — no native addon, no Python. It also boots
the metaharness kernel + Claude Code host adapter for the agent-harness commands (`init`/`doctor`).

## License

MIT © Ruvector Team
