# @ruvector/ruqu-wasm

[![npm](https://img.shields.io/npm/v/@ruvector/ruqu-wasm.svg)](https://www.npmjs.com/package/@ruvector/ruqu-wasm)
[![License: MIT](https://img.shields.io/badge/License-MIT-22d3ee.svg)](https://github.com/ruvnet/ruqu)

**State-vector quantum circuit simulation in WebAssembly.** This package contains the Rust `ruqu-core` simulator compiled to WASM. It has a circuit builder, a `simulate()` function, Grover search, QAOA MaxCut, and the bounded research JSON API that the [ruqu Explorer](https://ruvnet.github.io/ruqu/explorer/) uses. It runs in browsers and in Node.js, needs no server, and has no runtime dependencies.

Every JavaScript example in this README is run in CI against the package files, and its printed output is compared with the output shown. The script is [`scripts/verify-readme-examples.cjs`](https://github.com/ruvnet/ruqu/blob/main/scripts/verify-readme-examples.cjs).

## Install

```bash
npm install @ruvector/ruqu-wasm
```

## Quick start (Node.js)

The package is an ES module built for the `web` target. In Node.js, load the `.wasm` bytes yourself and call `initSync`, because the default `init()` uses `fetch`, and Node's `fetch` cannot read `file:` URLs.

```js
import { readFileSync } from 'node:fs';
import { initSync, WasmQuantumCircuit, simulate } from '@ruvector/ruqu-wasm';

const wasmUrl = new URL(import.meta.resolve('@ruvector/ruqu-wasm/ruqu_wasm_bg.wasm'));
initSync({ module: readFileSync(wasmUrl) });

// Bell state: H on qubit 0, then CNOT 0 -> 1.
const circuit = new WasmQuantumCircuit(2);
circuit.h(0);
circuit.cnot(0, 1);

const result = simulate(circuit);
console.log(result.probabilities.map((p) => p.toFixed(3)).join(' '));
console.log(`qubits=${result.num_qubits} gates=${result.gate_count} depth=${circuit.depth}`);
circuit.free();
```

```text
0.500 0.000 0.000 0.500
qubits=2 gates=2 depth=2
```

## Quick start (browser)

In a browser, or any bundler that serves the `.wasm` file next to `ruqu_wasm.js`, call the **default export** once before anything else:

```javascript
import init, { WasmQuantumCircuit, simulate } from '@ruvector/ruqu-wasm';

await init(); // fetches ruqu_wasm_bg.wasm relative to ruqu_wasm.js

const circuit = new WasmQuantumCircuit(3);
circuit.h(0);
circuit.cnot(0, 1);
circuit.cnot(1, 2); // GHZ state

const { probabilities } = simulate(circuit);
console.log(`P(000)=${probabilities[0].toFixed(3)} P(111)=${probabilities[7].toFixed(3)}`);
```

```text
P(000)=0.500 P(111)=0.500
```

Use the default export (`import init from ...`) to initialize. The package also has a named export `init`, which is the module's internal start hook. It does not load the WASM, so `import { init }` will not work as an initializer.

## Behaviour you need to know

### Qubit 0 is the rightmost bit

`probabilities[i]` is the probability of basis state `i`, and qubit `k` is bit `k` of `i`. So qubit 0 is the **least significant (rightmost)** bit when you write `i` in binary.

```js
import { readFileSync } from 'node:fs';
import { initSync, WasmQuantumCircuit, simulate } from '@ruvector/ruqu-wasm';
initSync({ module: readFileSync(new URL(import.meta.resolve('@ruvector/ruqu-wasm/ruqu_wasm_bg.wasm'))) });

const circuit = new WasmQuantumCircuit(3);
circuit.x(0); // flip qubit 0 only

const probs = simulate(circuit).probabilities;
const index = probs.indexOf(1);
console.log(`index=${index} bits=|${index.toString(2).padStart(3, '0')}>`);
```

```text
index=1 bits=|001>
```

### Measurement collapses at random unless you pass a seed

`measure(q)` and `measure_all()` collapse the state during `simulate()`. Each entry in `measurements` holds the classical `result` and the `probability` of that outcome **before** the collapse. The returned `probabilities` describe the state **after** the collapse.

If you call `simulate(circuit)` without a seed, the collapse uses OS entropy, so two runs can give different results. Pass an integer seed (`0` to `Number.MAX_SAFE_INTEGER`) as the second argument to make the result reproducible. A fractional, negative, or non-finite seed throws.

```js
import { readFileSync } from 'node:fs';
import { initSync, WasmQuantumCircuit, simulate } from '@ruvector/ruqu-wasm';
initSync({ module: readFileSync(new URL(import.meta.resolve('@ruvector/ruqu-wasm/ruqu_wasm_bg.wasm'))) });

const circuit = new WasmQuantumCircuit(2);
circuit.h(0);
circuit.cnot(0, 1);
circuit.measure(0);

const a = simulate(circuit, 7);
const b = simulate(circuit, 7);
console.log('same outcome:', a.measurements[0].result === b.measurements[0].result);
console.log('pre-collapse probability:', a.measurements[0].probability.toFixed(3));
console.log('collapsed:', a.probabilities.filter((p) => p > 0.999).length === 1);

try {
  simulate(circuit, 1.5);
} catch (e) {
  console.log('bad seed rejected');
}
```

```text
same outcome: true
pre-collapse probability: 0.500
collapsed: true
bad seed rejected
```

To get many shots, leave out `measure` and sample from `probabilities` with your own seeded PRNG. That needs one simulation, not one per shot. `npx ruvector quantum` does exactly this.

### Non-finite angles throw

`rx`, `ry`, `rz` and `rzz` throw if the angle is `NaN`, `Infinity` or `-Infinity`. In 2.1.0 and earlier, the circuit accepted the bad angle, and `simulate()` then returned all-`NaN` probabilities with no error.

```js
import { readFileSync } from 'node:fs';
import { initSync, WasmQuantumCircuit } from '@ruvector/ruqu-wasm';
initSync({ module: readFileSync(new URL(import.meta.resolve('@ruvector/ruqu-wasm/ruqu_wasm_bg.wasm'))) });

const circuit = new WasmQuantumCircuit(1);
try {
  circuit.rx(0, Number.NaN);
} catch (e) {
  console.log(String(e));
}
console.log('gates added:', circuit.gate_count);
```

```text
rx: angle must be a finite number, got NaN
gates added: 0
```

### Other details

- Errors are thrown as strings, not `Error` objects. Two examples: `new WasmQuantumCircuit(26)` throws `Qubit limit exceeded: 26 requested, max 25 in WASM`, and a gate on a qubit that does not exist is caught by `simulate()`.
- `gate_count` counts unitary gates only. `measure`, `reset` and `barrier` are not counted.
- `probabilities` is a plain JavaScript `Array`, not a typed array.
- Memory is `2^n * 16` bytes for `n` qubits: `estimate_memory(20)` is 16 MiB and `estimate_memory(25)` is 512 MiB. `max_qubits()` returns 25. For large circuits, run the simulation in a Web Worker.
- `WasmQuantumCircuit` holds WASM memory. Call `circuit.free()` when you are done with it, or use `using` where your runtime supports `Symbol.dispose`.

## Algorithms

`grover_search` and `qaoa_maxcut` take an optional seed: a number, `null` or `undefined`. A numeric seed is truncated to an unsigned 64-bit integer.

```js
import { readFileSync } from 'node:fs';
import { initSync, grover_search, qaoa_maxcut } from '@ruvector/ruqu-wasm';
initSync({ module: readFileSync(new URL(import.meta.resolve('@ruvector/ruqu-wasm/ruqu_wasm_bg.wasm'))) });

// Search 3 qubits (8 items) for |101> = 5.
const g = grover_search(3, new Uint32Array([5]), 42);
console.log(`grover: state=${g.measured_state} found=${g.target_found} iterations=${g.num_iterations} p=${g.success_probability.toFixed(4)}`);

// MaxCut on the path graph 0-1-2, one QAOA round.
const q = qaoa_maxcut(3, new Uint32Array([0, 1, 1, 2]), 1, new Float64Array([0.5]), new Float64Array([0.3]), 7);
console.log(`qaoa: states=${q.probabilities.length} expected_cut=${q.expected_cut.toFixed(4)}`);
```

```text
grover: state=5 found=true iterations=2 p=0.9453
qaoa: states=8 expected_cut=0.3960
```

## Research JSON API

`research_run_json` runs the same bounded research API as native Rust and the ruqu CLI. It covers magic-state factory, cultivation, Pauli propagation and hypergraph decoding. It takes a JSON string, returns a JSON string, and throws on invalid input or an exhausted budget. It never returns partial results. Seeds and large masks are decimal strings. See the [ruqu README](https://github.com/ruvnet/ruqu#one-research-api-across-rust-nodejs-and-browsers) for the list of operations.

```js
import { readFileSync } from 'node:fs';
import { initSync, research_run_json } from '@ruvector/ruqu-wasm';
initSync({ module: readFileSync(new URL(import.meta.resolve('@ruvector/ruqu-wasm/ruqu_wasm_bg.wasm'))) });

console.log(research_run_json(JSON.stringify({ op: 'magic-classify', mask: 7 })));
```

```text
{"accepted":true,"logical_error":true}
```

## API reference

The full declarations ship in `ruqu_wasm.d.ts`. CI checks that every line below appears in that file.

```ts
export class WasmQuantumCircuit {
    constructor(num_qubits: number);
    h(qubit: number): void;
    x(qubit: number): void;
    y(qubit: number): void;
    z(qubit: number): void;
    s(qubit: number): void;
    t(qubit: number): void;
    rx(qubit: number, angle: number): void;
    ry(qubit: number, angle: number): void;
    rz(qubit: number, angle: number): void;
    cnot(control: number, target: number): void;
    cz(q1: number, q2: number): void;
    swap(q1: number, q2: number): void;
    rzz(q1: number, q2: number, angle: number): void;
    measure(qubit: number): void;
    measure_all(): void;
    reset(qubit: number): void;
    barrier(): void;
    readonly num_qubits: number;
    readonly gate_count: number;
    readonly depth: number;
    free(): void;
}
export function simulate(circuit: WasmQuantumCircuit, seed?: number | null): any;
export function grover_search(num_qubits: number, target_states: Uint32Array, seed: any): any;
export function qaoa_maxcut(num_nodes: number, edges_flat: Uint32Array, p: number, gammas: Float64Array, betas: Float64Array, seed: any): any;
export function research_run_json(request: string): string;
export function estimate_memory(num_qubits: number): number;
export function max_qubits(): number;
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
```

Return shapes:

| Function | Returns |
| :--- | :--- |
| `simulate` | `{ probabilities: number[], measurements: { qubit, result: boolean, probability }[], num_qubits, gate_count, execution_time_ms }` |
| `grover_search` | `{ measured_state, target_found: boolean, success_probability, num_iterations }` |
| `qaoa_maxcut` | `{ probabilities: number[], expected_cut }` |

## Size

| File | Raw | gzip -9 |
| :--- | ---: | ---: |
| `ruqu_wasm_bg.wasm` | 405 KiB | 150 KiB |
| `ruqu_wasm.js` | 26 KiB | 6.3 KiB |

## Works with

- **[ruvector](https://www.npmjs.com/package/ruvector)**: `npx ruvector quantum simulate bell` runs this engine. It is an optional dependency of ruvector, and ruvector adds input validation and seeded shot sampling on top.
- **[@ruvector/ruqu](https://www.npmjs.com/package/@ruvector/ruqu)**: the ruqu CLI and agent harness (`npx @ruvector/ruqu`). It ships the same Rust engine built for the Node.js target.
- **[ruqu Explorer](https://ruvnet.github.io/ruqu/explorer/)**: the animated browser explorer, built on this package's web build.
- **Rust crates**: [`ruqu-core`](https://crates.io/crates/ruqu-core) and [`ruqu-algorithms`](https://crates.io/crates/ruqu-algorithms), the native simulator and algorithms that this package compiles.

## Changelog

- **2.2.0**: `rx`/`ry`/`rz`/`rzz` throw on non-finite angles. `simulate(circuit, seed?)` accepts an optional seed for reproducible measurement. The README was rewritten against the real API; the old `Circuit`/`Simulator` examples never existed in this package.

## License

MIT. Source: [github.com/ruvnet/ruqu](https://github.com/ruvnet/ruqu).
