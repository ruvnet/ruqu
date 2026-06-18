// ws.js — WebSocket telemetry client for the Sensing / Live Gate panel.
//
// Frame contract (one JSON text message per frame), a single SensorSyndrome:
//   { "source": string,
//     "detector_bits": [bool, ...],   // one bool per component label
//     "confidence": number,           // 0..1
//     "timestamp_ns": number }
//
// Component labels are fixed and known to the UI:
export const COMPONENT_LABELS = ["api", "db", "cache", "queue", "worker"];

// ---------------------------------------------------------------------------
// Pure frame generation (node --check / unit-testable: no DOM, no sockets).
// ---------------------------------------------------------------------------

/**
 * Tiny deterministic PRNG (mulberry32) so simulated runs are reproducible
 * and `node --check`-friendly (no Math.random dependence for shaping logic).
 */
export function makeRng(seed) {
  let a = seed >>> 0;
  return function next() {
    a |= 0;
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

/**
 * Build a single SensorSyndrome frame for a given regime.
 *
 * Regimes:
 *  - "correlated": several components trip together (broad, ambiguous failure)
 *      -> high syndrome entropy -> low coherence gate -> DEFER/DENY.
 *  - "isolated": one component clearly fails, others quiet
 *      -> concentrated -> high coherence -> PERMIT.
 *  - "quiet": nothing tripping -> uniform / undecided -> DEFER.
 *
 * @param {object} opts { regime, source, timestampNs, rng, labels }
 * @returns {{source:string, detector_bits:boolean[], confidence:number, timestamp_ns:number}}
 */
export function buildFrame({
  regime = "isolated",
  source = "sensor-0",
  timestampNs = Date.now() * 1e6,
  rng = Math.random,
  labels = COMPONENT_LABELS,
} = {}) {
  const n = labels.length;
  const bits = new Array(n).fill(false);
  let confidence = 0.9;

  if (regime === "correlated") {
    // Broad correlated failure: most components trip, with some jitter.
    for (let i = 0; i < n; i++) {
      bits[i] = rng() < 0.78;
    }
    // Guarantee at least 3 trips so the regime is clearly "wide".
    let trips = bits.filter(Boolean).length;
    while (trips < 3) {
      const i = Math.floor(rng() * n);
      if (!bits[i]) {
        bits[i] = true;
        trips++;
      }
    }
    confidence = 0.9 + rng() * 0.09;
  } else if (regime === "isolated") {
    // A single component clearly fails (rotates so the root cause moves).
    const idx = Math.floor(rng() * n);
    bits[idx] = true;
    // Occasional faint secondary blip (still concentrated).
    if (rng() < 0.2) {
      const j = (idx + 1) % n;
      bits[j] = rng() < 0.3;
    }
    confidence = 0.85 + rng() * 0.14;
  } else {
    // quiet: all clear.
    confidence = 0.92 + rng() * 0.07;
  }

  return {
    source,
    detector_bits: bits,
    confidence: Number(confidence.toFixed(4)),
    timestamp_ns: Math.round(timestampNs),
  };
}

/**
 * A stateful regime scheduler: alternates correlated <-> isolated regimes in
 * blocks so the live gauge visibly oscillates between DENY/DEFER and PERMIT.
 * Returns a function that yields the next frame each tick.
 *
 * @param {object} opts { seed, blockSize, labels }
 */
export function makeSimSource({ seed = 1337, blockSize = 6, labels = COMPONENT_LABELS } = {}) {
  const rng = makeRng(seed);
  // Cycle: isolated (PERMIT) -> correlated (DEFER/DENY) -> quiet (DEFER).
  const regimes = ["isolated", "isolated", "correlated", "correlated", "quiet"];
  let tick = 0;
  let sensorCounter = 0;

  return function nextFrame(nowNs) {
    const block = Math.floor(tick / blockSize) % regimes.length;
    const regime = regimes[block];
    tick++;
    const ts = nowNs != null ? nowNs : Date.now() * 1e6;
    const frame = buildFrame({
      regime,
      source: "sensor-" + (sensorCounter % 4),
      timestampNs: ts,
      rng,
      labels,
    });
    sensorCounter++;
    frame.regime = regime; // annotation for the UI (ignored by the gate API)
    return frame;
  };
}

// ---------------------------------------------------------------------------
// Live client (DOM/socket side).
// ---------------------------------------------------------------------------

export class TelemetryClient {
  constructor() {
    this._socket = null;
    this._simTimer = null;
    this._simSource = null;
    this._onFrame = null;
    this._onStatus = null;
    this._url = null;
    this._backoffMs = 500;
    this._maxBackoffMs = 8000;
    this._manualClose = false;
    this._mode = "idle"; // idle | live | simulated
  }

  /** Register a status callback: (state:string, detail?:string) => void */
  onStatus(cb) {
    this._onStatus = cb;
  }

  _emitStatus(state, detail) {
    this._mode = state === "simulated" ? "simulated" : this._mode;
    if (this._onStatus) this._onStatus(state, detail || "");
  }

  get mode() {
    return this._mode;
  }

  /**
   * Connect to a live WS URL. Falls back to the simulated source if the URL is
   * empty or the socket errors before/while connecting.
   */
  connect(url, onFrame) {
    this._onFrame = onFrame;
    this._manualClose = false;

    if (!url || !url.trim()) {
      this.useSimulated(onFrame);
      return;
    }

    this._url = url.trim();
    this._mode = "live";
    this._open();
  }

  _open() {
    this._emitStatus("connecting", this._url);
    let sock;
    try {
      sock = new WebSocket(this._url);
    } catch (err) {
      this._emitStatus("error", String(err));
      this._fallbackToSim();
      return;
    }
    this._socket = sock;

    sock.addEventListener("open", () => {
      this._backoffMs = 500;
      this._emitStatus("connected", this._url);
    });

    sock.addEventListener("message", (ev) => {
      let frame;
      try {
        frame = JSON.parse(ev.data);
      } catch {
        return; // ignore malformed frame
      }
      if (this._onFrame) this._onFrame(frame);
    });

    sock.addEventListener("error", () => {
      this._emitStatus("error", this._url);
      // error is usually followed by close; reconnect handled there.
    });

    sock.addEventListener("close", () => {
      this._socket = null;
      if (this._manualClose) {
        this._emitStatus("disconnected");
        return;
      }
      // Auto-reconnect with backoff; if we never connected, fall back to sim
      // after a couple of attempts so the panel is never dead.
      this._emitStatus("reconnecting", `retry in ${this._backoffMs}ms`);
      setTimeout(() => {
        if (this._manualClose) return;
        // After backoff has grown past threshold, fall back to simulated so
        // the live gauge always has data.
        if (this._backoffMs >= this._maxBackoffMs) {
          this._fallbackToSim();
          return;
        }
        this._backoffMs = Math.min(this._backoffMs * 2, this._maxBackoffMs);
        this._open();
      }, this._backoffMs);
    });
  }

  _fallbackToSim() {
    this._teardownSocket();
    this._emitStatus("simulated", "live source unavailable — using built-in feed");
    this._startSim();
  }

  /** Switch explicitly to the built-in simulated source. */
  useSimulated(onFrame) {
    this._onFrame = onFrame || this._onFrame;
    this._manualClose = false;
    this._teardownSocket();
    this._mode = "simulated";
    this._emitStatus("simulated", "built-in simulated feed");
    this._startSim();
  }

  _startSim() {
    this._stopSim();
    this._simSource = makeSimSource({ seed: (Date.now() & 0xffff) | 1 });
    this._simTimer = setInterval(() => {
      const frame = this._simSource(Date.now() * 1e6);
      if (this._onFrame) this._onFrame(frame);
    }, 900);
  }

  _stopSim() {
    if (this._simTimer != null) {
      clearInterval(this._simTimer);
      this._simTimer = null;
    }
    this._simSource = null;
  }

  _teardownSocket() {
    if (this._socket) {
      try {
        this._socket.onclose = null;
        this._socket.close();
      } catch {
        /* ignore */
      }
      this._socket = null;
    }
  }

  /** Stop everything (live socket + sim) and report disconnected. */
  disconnect() {
    this._manualClose = true;
    this._teardownSocket();
    this._stopSim();
    this._mode = "idle";
    this._emitStatus("disconnected");
  }
}
