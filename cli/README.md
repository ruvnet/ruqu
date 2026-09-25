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

### Boundary Observatory

`ruqu boundary-lab plan`, `run`, `predict`, `verify` and `project` implement an
offline synthetic RF experiment. Pass the hash from `plan` to
`run --protocol-hash HASH [--seed N --trials N]`. Trial count is per arm, from 100
through 500. `predict --file measurements.json` accepts only the strict synthetic
measurement contract. `verify` and `project` require `--file report.json` and a
separately retained `--trusted-root ROOT`.

The experiment uses eight perimeter sensors, at most one emitter per frequency,
a fixed inference grid and three control arms. Graph and field exports are local
contracts, not native RuField or WorldGraph serialization. No network, hardware
or automatic publishing is involved. See `examples/boundary-lab/README.md` in
the repository. This feature is not yet published to npm.

| Command | What it does |
|---|---|
| `simulate [--qubits N]` | Run a GHZ/Bell circuit on the WASM state-vector simulator (up to 25 qubits). |
| `grover [--qubits N --target T --seed S]` | Grover amplitude amplification / search. |
| `qaoa [--nodes N --p P]` | QAOA MaxCut on a ring graph. |
| `capabilities` | List gates, algorithms, qubit/memory limits. |
| `observer-lab [--count N --seed S]` | Emit reproducible simulated evidence and separately altered memories as JSON. Count 1..10000, seed 1..4294967295. |
| `observer-lab --benchmark` | Compare the original and optimized dephasing computation; local timings only. |
| `init` · `doctor` | Boot / verify the agent-harness kernel **and** the quantum WASM. |
| `version` | Kernel + WASM versions. |

Gates: `h x y z s t rx ry rz cnot cz swap rzz measure reset barrier`. Up to **25 qubits**.

## How it works

The CLI bundles a `--target nodejs` WebAssembly build of the `ruqu-wasm` crate (real state-vector
simulation in Rust → WASM) and loads it directly in Node — no native addon, no Python. It also boots
the metaharness kernel + Claude Code host adapter for the agent-harness commands (`init`/`doctor`).

The observer laboratory is available from this source checkout with
`node bin/cli.js observer-lab`. It uses no harness imports or network access.
It is a classical simulator of quantum predictions, not quantum hardware.
An optional Rust adapter in `integrations/observer-world` in the repository
constructs real ruField and WorldGraph types after evidence verification.
The adapter is not included in the npm package. No npm publication is part of
this change.

## License

MIT © Ruvector Team
