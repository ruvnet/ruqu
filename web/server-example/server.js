// Example WebSocket telemetry server for the ruQu management console (ADR-258).
//
// Streams one `SensorSyndrome` JSON frame per tick (default ~500ms) to every
// connected client. The frame contract matches the console's Sensing / Live
// Gate panel exactly:
//
//   {
//     "source":        string,        // origin of the syndrome
//     "detector_bits": [bool, ...],   // length 5: [api, db, cache, queue, worker]
//     "confidence":    number,        // 0..1
//     "timestamp_ns":  number         // wall-clock nanoseconds
//   }
//
// bit i == true means component i is anomalous, using the fixed 5-component
// layout ["api", "db", "cache", "queue", "worker"].
//
// The stream alternates between two regimes so the live console gauge visibly
// moves:
//   * CORRELATED  — api + db + queue fire together at high confidence. The
//                   console reads this as low coherence -> DENY / DEFER.
//   * QUIET       — mostly zeros with the occasional isolated bit and lower
//                   confidence. High coherence -> PERMIT.
//
// Configuration via environment variables:
//   PORT          TCP port to listen on              (default 8787)
//   TICK_MS       milliseconds between frames         (default 500)
//   REGIME_MS     milliseconds per regime before flip (default 6000)

import { WebSocketServer } from "ws";

const PORT = Number(process.env.PORT) || 8787;
const TICK_MS = Number(process.env.TICK_MS) || 500;
const REGIME_MS = Number(process.env.REGIME_MS) || 6000;

// Fixed 5-component layout — must match the console's labels exactly.
const COMPONENTS = ["api", "db", "cache", "queue", "worker"];

// Deterministic-ish but lively PRNG (mulberry32) so the feed looks alive while
// staying reproducible across runs.
function makeRng(seed) {
  let a = seed >>> 0;
  return function next() {
    a |= 0;
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const rng = makeRng(0x5e7501);

function nowNs() {
  // High-resolution wall-clock nanoseconds.
  return BigInt(Date.now()) * 1_000_000n + BigInt(Math.floor(rng() * 1_000_000));
}

// Build a syndrome frame for the given regime.
function buildSyndrome(regime) {
  const bits = [false, false, false, false, false];
  let confidence;
  let source;

  if (regime === "correlated") {
    // api(0) + db(1) + queue(3) fire together — a correlated failure cascade.
    bits[0] = true;
    bits[1] = true;
    bits[3] = true;
    // Occasionally drag the worker in too, for variety.
    if (rng() < 0.4) bits[4] = true;
    confidence = 0.85 + rng() * 0.13; // 0.85..0.98
    source = "telemetry/correlated";
  } else {
    // Quiet regime: mostly zeros, occasional single isolated anomaly.
    if (rng() < 0.35) {
      const i = Math.floor(rng() * COMPONENTS.length);
      bits[i] = true;
    }
    confidence = 0.55 + rng() * 0.25; // 0.55..0.80
    source = "telemetry/quiet";
  }

  return {
    source,
    detector_bits: bits,
    confidence: Number(confidence.toFixed(4)),
    timestamp_ns: Number(nowNs()),
  };
}

const wss = new WebSocketServer({ port: PORT });

// Alternate regimes on a timer shared across all clients.
let regime = "quiet";
setInterval(() => {
  regime = regime === "quiet" ? "correlated" : "quiet";
}, REGIME_MS);

// Broadcast one frame per tick to all connected clients.
setInterval(() => {
  if (wss.clients.size === 0) return;
  const frame = JSON.stringify(buildSyndrome(regime));
  for (const client of wss.clients) {
    if (client.readyState === client.OPEN) {
      client.send(frame);
    }
  }
}, TICK_MS);

wss.on("connection", (socket, req) => {
  const peer = req.socket.remoteAddress || "unknown";
  console.log(`[ruqu-sensing] client connected: ${peer} (clients=${wss.clients.size})`);

  // Send one frame immediately so the panel populates without waiting a tick.
  socket.send(JSON.stringify(buildSyndrome(regime)));

  socket.on("close", () => {
    console.log(`[ruqu-sensing] client disconnected (clients=${wss.clients.size})`);
  });

  socket.on("error", (err) => {
    console.error(`[ruqu-sensing] socket error: ${err.message}`);
  });
});

wss.on("listening", () => {
  console.log(
    `[ruqu-sensing] SensorSyndrome stream on ws://localhost:${PORT} ` +
      `(tick=${TICK_MS}ms, regime=${REGIME_MS}ms, layout=[${COMPONENTS.join(", ")}])`
  );
  console.log("[ruqu-sensing] point the console Sensing panel at this URL.");
});

process.on("SIGINT", () => {
  console.log("\n[ruqu-sensing] shutting down.");
  wss.close(() => process.exit(0));
});
