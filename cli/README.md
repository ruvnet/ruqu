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
| `observer-lab [--count N --seed S]` | Emit reproducible simulated evidence and separately altered memories as JSON. Count 1..10000, seed 1..4294967295. |
| `observer-lab --benchmark` | Compare the original and optimized dephasing computation; local timings only. |
| `init` · `doctor` | Boot / verify the agent-harness kernel **and** the quantum WASM. |
| `magic-factory [--input-error P --shots N --seed N --rounds N]` | Exact ideal 15-to-1 model, deterministic sampling and retry resource accounting. Source checkout. |
| `magic-factory --research` | Cited cultivation and biased-noise profiles with missing costs left unknown. |
| `magic-factory --benchmark` | Bounded synthetic numerical checks. |
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

## Magic state factory

```bash
node bin/cli.js magic-factory --input-error 0.01 --shots 100000 --seed 42
node bin/cli.js magic-factory --input-error 0.01 --rounds 2 --transfer-acceptance 0.9 --transfer-error 0.000001 --transfer-qubits 20 --transfer-rounds 5
node bin/cli.js magic-factory --research
```

The new command is available from this checkout. It uses no provider, network or WASM imports. Sampling accepts 1 to 5,000,000 shots and unsigned 64 bit decimal seeds. Resource estimates accept 1 to 16 distillation rounds. `--block-qubits`, `--block-rounds`, `--raw-qubits`, and `--raw-rounds` set caller costs; defaults are illustrative. Transfer error bounds include possible concentration of upstream errors after selection. Qubit rounds are expected consumed volume, not peak qubits or latency.

The model assumes independent input Z faults and perfect Clifford operations. It does not reproduce a physical cultivation circuit. Rust/Node agreement is checked by `node scripts/verify-research.cjs` from the repository root.

## License

MIT © Ruvector Team
