# Example WebSocket telemetry server

A tiny Node WebSocket server that feeds the ruQu management console's
**Sensing / Live Gate** panel with `SensorSyndrome` frames (ADR-258). Use it to
exercise the live coherence gauge locally — GitHub Pages is static-only and
cannot host this server, so the deployed console relies on an external
`ws://`/`wss://` endpoint like this one (or its built-in simulated fallback).

## Run

```bash
cd web/server-example
npm install      # installs the single dependency: ws
npm start        # serves ws://localhost:8787
```

Configuration via environment variables:

| Variable    | Default | Meaning                                   |
|-------------|---------|-------------------------------------------|
| `PORT`      | `8787`  | TCP port to listen on                     |
| `TICK_MS`   | `500`   | Milliseconds between frames               |
| `REGIME_MS` | `6000`  | Milliseconds per regime before flipping   |

```bash
PORT=9001 TICK_MS=250 REGIME_MS=4000 npm start
```

## Frame format

Each WebSocket text message is a single JSON `SensorSyndrome`:

```json
{
  "source": "telemetry/correlated",
  "detector_bits": [true, true, false, true, false],
  "confidence": 0.93,
  "timestamp_ns": 1718668800123000000
}
```

- `source` — string label for where the syndrome came from.
- `detector_bits` — fixed-length-5 boolean array using the layout
  `["api", "db", "cache", "queue", "worker"]`; `bit[i] === true` means
  component `i` is anomalous.
- `confidence` — number in `0..1`.
- `timestamp_ns` — wall-clock nanoseconds.

The stream alternates between two regimes so the gauge visibly moves:

- **CORRELATED** — `api` + `db` + `queue` fire together at high confidence. The
  console reads this as low coherence → **DENY / DEFER**.
- **QUIET** — mostly zeros with the occasional isolated bit and lower
  confidence → high coherence → **PERMIT**.

## Point the console at it

1. Build and serve the console (see [`../README.md`](../README.md)):
   ```bash
   wasm-pack build crates/ruqu-console-wasm --target web --out-dir web/pkg --out-name ruqu_console
   python3 -m http.server 8099 --directory web
   ```
2. Open <http://localhost:8099/>.
3. In the **Sensing / Live Gate** panel, set the WebSocket endpoint to
   `ws://localhost:8787` and connect.

The deployed (GitHub Pages) console can point at any reachable endpoint; for a
page served over HTTPS you must use a secure `wss://` URL.
