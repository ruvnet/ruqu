# ruqu-console-wasm — WebAssembly bindings for the management console

Browser-facing WASM API for the ruQu **Structural Possibility Runtime**
([ADR-258](../../docs/adr/ADR-258-structural-possibility-runtime.md)). A single
module that drives the whole runtime client-side, so the management console
([`web/`](../../web)) is a thin view over the real Rust logic — no server
required (GitHub Pages is static-only).

## Build

```bash
wasm-pack build crates/ruqu-console-wasm --target web --out-dir web/pkg --out-name ruqu_console
```

This emits `web/pkg/ruqu_console.js` + `ruqu_console_bg.wasm` (~512 KB) which the
console imports directly.

## API

Every function takes JSON strings and returns plain JS objects
(`serde-wasm-bindgen`); decisions and receipts come back exactly as the native
runtime emits them.

| Export | Purpose |
|--------|---------|
| `version()` | crate versions |
| `analyze_field(candidatesJson, seed)` | possibility-field entropy/coherence/gate + collapse receipt |
| `rag_search(queryJson, corpusJson, k, rounds, phaseKickback, seed)` | interference reranking vs cosine baseline |
| `swarm_consensus(wavefrontJson, seed)` | swarm collapse consensus + reasoning-QEC receipt |
| `sensing_diagnose(topologyJson, faultRate, rounds, seed)` | structural fault localization |
| `gate_from_syndromes(syndromesJson, labelsJson, seed)` | live syndrome → coherence-gated root cause (WebSocket path) |
| `syndromes_from_samples(channelsJson, samplesJson)` | telemetry → `SensorSyndrome[]` |
| `quantum_ghz(n)`, `quantum_grover(n, target, seed)` | in-browser quantum-circuit demos |
| `WasmReceiptLog` | stateful, hash-chained, verifiable receipt log |

```js
import init, { rag_search } from './pkg/ruqu_console.js';
await init();
const r = rag_search(JSON.stringify([1,0,0]), corpusJson, 3, 3, true, 42);
// r.cosine_top_k[0] is a contradicted doc; r.selected[0] is the coherent one.
```

MIT © Ruvector Team.
