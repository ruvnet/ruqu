# ruQu management console (web)

A static, browser-based **management console** for the ruQu structural
possibility runtime (ADR-258). It is a plain static web app — HTML, CSS, and
ES modules — driven by a WebAssembly module compiled from the
`ruqu-console-wasm` crate. There is no JavaScript bundler or app build step;
the only build artifact is the WASM bundle produced by `wasm-pack`.

The console visualizes possibility fields, coherence gating
(PERMIT / DEFER / DENY), collapse receipts, and a live **Sensing / Live Gate**
panel that consumes a WebSocket telemetry stream.

### Clifford (large-N) panel

The **Clifford (large-N)** panel exposes ruQu's Aaronson–Gottesman *stabilizer*
simulator (`ruqu_core::stabilizer`) directly in the browser. Where the dense
state-vector demos in the **Quantum** panel allocate a `2^n` amplitude array —
and so hit a hard memory wall around **25 qubits** in the browser — a Clifford
circuit is stored as an `O(n^2)`-bit tableau. Memory therefore grows
*polynomially*, letting the console run circuits with **thousands of qubits**
client-side (the WASM bindings cap a single call at 16384 qubits).

The panel offers two seeded, deterministic runners over the
`clifford_ghz(num_qubits, seed)` and `clifford_random(num_qubits, depth, seed)`
WASM exports:

- **GHZ state (large N)** — prepares an `n`-qubit GHZ state and measures every
  qubit; a correct collapse makes all measured bits equal (`all_equal: true`).
- **Random Clifford circuit** — applies `depth` layers of random Clifford gates
  (H/S/X/Y/Z plus CNOT/CZ pairs) and measures all qubits.

Both report the qubit count, elapsed time, the result, and a short bit sample.
The trade-off is that this engine is restricted to the Clifford gate set; for
universal (non-Clifford) circuits use the state-vector **Quantum** panel within
its ~25-qubit limit.

## Layout

```
web/
├── index.html          # console shell (owned elsewhere)
├── styles.css          # styles (owned elsewhere)
├── js/                 # app ES modules (owned elsewhere)
├── pkg/                # generated WASM bundle — gitignored, build it fresh
├── server-example/     # example WebSocket telemetry server (this PR)
└── README.md           # this file
```

## Build the WASM bundle

The app imports `./pkg/ruqu_console.js`. Generate it with `wasm-pack`:

```bash
# from the repository root
wasm-pack build crates/ruqu-console-wasm --target web --out-dir web/pkg --out-name ruqu_console
```

This writes `ruqu_console.js`, `ruqu_console_bg.wasm`, the `.d.ts` typings, a
`package.json`, and a `.gitignore` into `web/pkg/`.

> **Note:** `web/pkg/` is generated and **gitignored**. It is not committed; CI
> (and you, locally) must build it before serving. If the console fails to load,
> the most common cause is a missing `web/pkg/` — run the command above.

## Serve locally

Any static file server works. With Python:

```bash
python3 -m http.server 8099 --directory web
```

Then open <http://localhost:8099/>.

## WebSocket integration (Sensing / Live Gate)

The Sensing panel consumes a WebSocket stream of `SensorSyndrome` frames. Each
text message is a single JSON object with a **fixed 5-component layout**
(`["api", "db", "cache", "queue", "worker"]`):

```json
{
  "source": "telemetry/correlated",
  "detector_bits": [true, true, false, true, false],
  "confidence": 0.93,
  "timestamp_ns": 1718668800123000000
}
```

- `detector_bits` has length 5; `bit[i] === true` means component `i` is
  anomalous.

To drive the panel locally, run the bundled example server and point the panel
at `ws://localhost:8787`:

```bash
cd web/server-example
npm install
npm start
```

See [`server-example/README.md`](server-example/README.md) for details, the
regime behaviour (correlated → DENY/DEFER, quiet → PERMIT), and configuration.

When no live endpoint is available, use the console's **built-in simulated
feed**. This is the only option on GitHub Pages, which is static-only — see
below.

## GitHub Pages deployment

The console is deployed to GitHub Pages by
[`.github/workflows/pages.yml`](../.github/workflows/pages.yml). On every push
to the default branch (and via **Run workflow**), CI:

1. Checks out the repo and installs Rust with the `wasm32-unknown-unknown`
   target.
2. Installs `wasm-pack` and runs the build command above, producing a fresh
   `web/pkg/` (needed because it is gitignored).
3. Uploads the whole `web/` directory as the Pages artifact and deploys it with
   `actions/deploy-pages`.

**One-time repo setup:** GitHub → **Settings → Pages → Build and deployment →
Source: GitHub Actions**.

### Static-hosting constraint

GitHub Pages serves static files only — **there is no server on Pages**. The
WebSocket telemetry server cannot run there. The deployed console therefore
either:

- points its Sensing panel at an **external** `ws://` / `wss://` endpoint you
  control (a page served over HTTPS requires a secure `wss://` URL), or
- uses the **built-in simulated feed** for a fully client-side demo.
