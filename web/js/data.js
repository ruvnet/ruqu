// data.js — pure default datasets and small data-shaping helpers used by the
// panels. No DOM, no WASM: everything here is `node --check`-able and easy to
// unit-test.

// ---------------------------------------------------------------------------
// Possibility Field defaults: 2 coherent candidates (phase ~0) + 1 contradicted
// (phase ~pi) so the default field collapses to DENY.
// ---------------------------------------------------------------------------
export function defaultCandidates() {
  return [
    { id: "deploy-canary", amplitude: 0.9, phase: 0.05 },
    { id: "deploy-bluegreen", amplitude: 0.72, phase: 0.12 },
    { id: "rollback-now", amplitude: 0.85, phase: Math.PI }, // contradicted
  ];
}

// ---------------------------------------------------------------------------
// Interference RAG corpus: ~6 short docs. `cache-ttl-stale` has a very high
// cosine to the query but is heavily contradicted + stale, so plain cosine
// ranks it #1 while interference suppresses it to the bottom.
// Embeddings are 4-dim toy vectors aligned to topic axes: [cache, db, queue, auth]
// ---------------------------------------------------------------------------
export function defaultRagQuery() {
  // "What is the current cache TTL?" -> cache axis.
  return [0.97, 0.12, 0.06, 0.04];
}

export function defaultRagCorpus() {
  return [
    {
      id: "cache-ttl-current",
      text: "Cache TTL is 60 seconds as of the latest infra config.",
      embedding: [0.95, 0.1, 0.05, 0.03],
      source_trust: 0.92,
      recency: 0.95,
      graph_proximity: 0.8,
      contradiction: 0.0,
      novelty: 0.3,
    },
    {
      id: "cache-ttl-stale",
      text: "Cache TTL is 3600 seconds. (Outdated runbook, contradicted by current config.)",
      embedding: [0.99, 0.08, 0.04, 0.02], // even higher raw cosine
      source_trust: 0.25,
      recency: 0.05,
      graph_proximity: 0.5,
      contradiction: 0.95, // strongly contradicted
      novelty: 0.1,
    },
    {
      id: "cache-eviction",
      text: "Cache uses LRU eviction with a 2GB ceiling.",
      embedding: [0.82, 0.18, 0.1, 0.05],
      source_trust: 0.85,
      recency: 0.7,
      graph_proximity: 0.65,
      contradiction: 0.0,
      novelty: 0.5,
    },
    {
      id: "db-pool",
      text: "Database connection pool max is 20 per node.",
      embedding: [0.1, 0.94, 0.08, 0.05],
      source_trust: 0.88,
      recency: 0.75,
      graph_proximity: 0.6,
      contradiction: 0.0,
      novelty: 0.4,
    },
    {
      id: "queue-retry",
      text: "Queue retries failed jobs up to 3 times with backoff.",
      embedding: [0.06, 0.12, 0.95, 0.04],
      source_trust: 0.8,
      recency: 0.6,
      graph_proximity: 0.4,
      contradiction: 0.0,
      novelty: 0.55,
    },
    {
      id: "auth-token",
      text: "Auth tokens expire after 15 minutes; refresh rotates them.",
      embedding: [0.04, 0.06, 0.05, 0.96],
      source_trust: 0.9,
      recency: 0.85,
      graph_proximity: 0.3,
      contradiction: 0.0,
      novelty: 0.6,
    },
  ];
}

// ---------------------------------------------------------------------------
// Swarm consensus preset: 7 agents, 3 plans, votes leaning to one plan.
// ---------------------------------------------------------------------------
export function defaultWavefront() {
  return {
    agents: [
      { id: "architect", role: "planner", confidence: 0.9 },
      { id: "sre", role: "reliability", confidence: 0.85 },
      { id: "security", role: "critic", confidence: 0.8 },
      { id: "qa", role: "verifier", confidence: 0.78 },
      { id: "product", role: "stakeholder", confidence: 0.7 },
      { id: "oncall", role: "operator", confidence: 0.82 },
      { id: "data", role: "analyst", confidence: 0.75 },
    ],
    plans: [
      {
        id: "ship-canary",
        description: "Roll out canary to 5% then ramp on green metrics.",
        evidence_support: 0.86,
        steps: ["deploy canary", "watch SLOs 30m", "ramp to 100%"],
      },
      {
        id: "full-rollout",
        description: "Deploy to 100% immediately to hit the deadline.",
        evidence_support: 0.55,
        steps: ["deploy all", "monitor"],
      },
      {
        id: "hold-release",
        description: "Hold the release pending another review cycle.",
        evidence_support: 0.5,
        steps: ["freeze", "schedule review"],
      },
    ],
    votes: [
      { agent_id: "architect", plan_id: "ship-canary", confidence: 0.9, support: true },
      { agent_id: "sre", plan_id: "ship-canary", confidence: 0.88, support: true },
      { agent_id: "qa", plan_id: "ship-canary", confidence: 0.8, support: true },
      { agent_id: "oncall", plan_id: "ship-canary", confidence: 0.82, support: true },
      { agent_id: "data", plan_id: "ship-canary", confidence: 0.74, support: true },
      { agent_id: "product", plan_id: "full-rollout", confidence: 0.7, support: true },
      { agent_id: "security", plan_id: "hold-release", confidence: 0.8, support: true },
    ],
    consensus_threshold: 0.6,
  };
}

// ---------------------------------------------------------------------------
// Sensing topology preset (<= 25 components+connections). Connections use
// numeric component indices [fromIdx, toIdx, strength].
// ---------------------------------------------------------------------------
export function defaultTopology() {
  const components = ["api", "db", "cache", "queue", "worker", "lb", "auth"];
  return {
    components,
    health: [0.9, 0.45, 0.82, 0.6, 0.7, 0.95, 0.88],
    connections: [
      [5, 0, 0.95], // lb -> api
      [0, 1, 0.9], // api -> db
      [0, 2, 0.7], // api -> cache
      [0, 6, 0.6], // api -> auth
      [1, 3, 0.55], // db -> queue
      [3, 4, 0.8], // queue -> worker
      [4, 1, 0.5], // worker -> db
    ],
  };
}

// ---------------------------------------------------------------------------
// Live gate: derive a fixed-width detector_bits row from a frame, padded /
// truncated to the label count.
// ---------------------------------------------------------------------------
export function normalizeFrameBits(frame, labelCount) {
  const bits = Array.isArray(frame && frame.detector_bits) ? frame.detector_bits.slice(0, labelCount) : [];
  while (bits.length < labelCount) bits.push(false);
  return bits.map((b) => !!b);
}

/** Map a 0..1 coherence to a gate-band token for sparkline dot coloring. */
export function bandForGate(gate) {
  const g = String(gate || "").toUpperCase();
  if (g.includes("PERMIT")) return "permit";
  if (g.includes("DEFER")) return "defer";
  if (g.includes("DENY")) return "deny";
  return "unknown";
}
