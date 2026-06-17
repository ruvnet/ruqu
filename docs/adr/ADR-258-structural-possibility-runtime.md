# ADR-258: ruQu Structural Possibility Runtime for Agentic Intelligence

- **Status**: Proposed
- **Date**: 2026-06-17
- **Deciders**: ruv
- **Related systems**: ruQu, RuVector, RuFlo, emergent-time, ruv-neural, RuView, MetaHarness
- **Supersedes / extends**: ADR-257 (ruqu extraction), ADR-001 (metaharness CLI)

## 1. Context

`ruqu` is already more than a quantum simulator. It is a pure-Rust + WebAssembly
quantum computing toolkit with a state-vector circuit simulator, SIMD
acceleration, noise models, VQE, Grover, QAOA, surface-code error correction, and
a real-time coherence engine. The workspace ships the crates `ruqu-core`,
`ruqu-algorithms`, `ruqu-exotic`, `ruqu-wasm`, and `ruqu` (the coherence gate).

The strongest strategic opportunity is to treat ruQu as a **structural
possibility runtime** for AI systems: using quantum simulation, interference
search, error correction, coherence gating, and reversible memory as operational
primitives for agents, retrieval, sensing, forecasting, governance, and
scientific discovery.

The `ruqu-exotic` crate already points directly at this direction with quantum
memory decay, interference search, reasoning error correction, swarm
interference, syndrome diagnosis, and reversible memory for AI systems.

## 2. Problem

Current AI systems usually decide through a brittle pipeline:

```
retrieve → rank → reason → act → log
```

The failure modes are predictable: weak retrieval, premature commitment, agent
disagreement, hidden contradiction, untracked uncertainty, and non-auditable
state changes.

We need a runtime that can **hold multiple plausible states, amplify coherent
evidence, suppress incoherent paths, detect structural risk before action, and
emit receipts.**

## 3. Decision

Build the **ruQu Structural Possibility Runtime**: a layer that lets AI systems
model, score, collapse, and audit *possibility fields*. It exposes six
primitives:

1. **Possibility field** — competing hypotheses, retrieval candidates, agent
   plans, sensor interpretations, or engineering designs.
2. **Interference scoring** — amplify candidates that reinforce each other,
   suppress candidates that contradict the field.
3. **Coherence gate** — convert structural risk into `PERMIT`, `DEFER`, `DENY`.
4. **Reasoning error correction** — detect inconsistent reasoning chains via
   redundancy, syndrome bits, and verifier passes.
5. **Reversible memory** — reverse, replay, and audit agent / retrieval /
   decision state.
6. **Collapse receipt** — record *why* one path was selected over the others.

## 4. Why ruQu is the right base

The substrate already exists in the workspace:

| Crate | Role in the runtime |
|-------|---------------------|
| `ruqu-core` | circuit construction, state evolution, stabilizer & tensor-network simulation, SIMD kernels, `Complex` amplitude type |
| `ruqu-algorithms` | VQE, Grover, QAOA, surface code → search, optimization, constrained planning |
| `ruqu-exotic` | the directly relevant package: `interference_search`, `reasoning_qec`, `swarm_interference`, `syndrome_diagnosis`, `reversible_memory`, `quantum_decay`, `quantum_collapse` |
| `ruqu-wasm` | browser / Node execution (~25-qubit) |
| `ruqu` | the coherence gate: real-time structural health via dynamic min-cut → `PERMIT`/`DEFER`/`DENY` |

## 5. Core architecture

```
User query or event
      │
      ▼
RuFlo agent harness
      │
      ▼
RuVector memory recall
      │
      ▼
ruQu possibility field        (ruqu-possibility)
      │
      ▼
Interference scoring          (ruqu-rag / ruqu-agent over ruqu-exotic)
      │
      ▼
Coherence gate                (ruqu-possibility CoherenceGate / ruqu fabric)
      │
      ▼
Reasoning correction          (ruqu-exotic reasoning_qec)
      │
      ▼
Collapse receipt              (ruqu-possibility / ruqu-receipts)
      │
      ▼
Action, answer, alert, or design
```

## 6. Runtime packages

### 6.1 Existing crates

`ruqu-core`, `ruqu-algorithms`, `ruqu-exotic`, `ruqu-wasm`, `ruqu` — unchanged,
additive only.

### 6.2 New crates introduced by this ADR

| Crate | Purpose | Phase |
|-------|---------|-------|
| `ruqu-possibility` | common possibility-field abstraction: `PossibilityField`, `Possibility`, `CoherenceGate`, `CollapseReceipt`, `EvidenceReceipt`, `PossibilityRuntime` | 1 |
| `ruqu-rag` | interference reranking — possibility-field retrieval with collapse receipts | 2 |
| `ruqu-agent` | swarm collapse consensus — interference consensus + reasoning QEC + receipts | 3 |
| `ruqu-sensing` | telemetry → syndrome streams → structural anomaly detection | (sensing) |
| `ruqu-receipts` | tamper-evident collapse/replay logs and governance evidence | (governance) |

All new crates are `0.1.0`, edition 2021, Rust 1.77, MIT, dependency-light, and
build natively. They depend on `ruqu-possibility` plus the relevant existing
crates.

## 7. ⚠️ Aspirational sketches vs. the real API

**Important for implementers and reviewers.** The original ADR draft contained
illustrative code (e.g. `InterferenceIndex::new(384)`, `Query::new(...).interference_rounds(3)`,
`ReasoningCode::new().redundancy(3)`, `ReversibleStore`, `SwarmState::new(n)`,
`Diagnostics::new().stabilizers(...)`, `FabricBuilder::new().num_tiles(256)`).
These were **design intent, not the shipped API.** The real `ruqu-exotic` /
`ruqu` surfaces are different and the implementation wraps the *real* ones:

| Aspirational sketch | Real API used by this runtime |
|---------------------|-------------------------------|
| `InterferenceIndex` / `Query` | `ruqu_exotic::interference_search::{ConceptSuperposition, InterferenceScore, interference_search}` |
| `ReasoningCode` / `LogicalChain` | `ruqu_exotic::reasoning_qec::{ReasoningStep, ReasoningQecConfig, ReasoningTrace::run_qec}` |
| `SwarmState::new(n)` | `ruqu_exotic::swarm_interference::{Action, AgentContribution, SwarmInterference::decide}` |
| `Diagnostics` | `ruqu_exotic::syndrome_diagnosis::{Component, Connection, SystemDiagnostics::diagnose}` |
| `ReversibleStore` / `Operation` | `ruqu_exotic::reversible_memory::ReversibleMemory` (gate-level `apply`/`rewind`/`counterfactual`) |
| `FabricBuilder::new().num_tiles` | `ruqu::{FabricBuilder, CoherenceGate, GateDecision}` (real builder is `.tiles()/.syndrome_buffer()/.build()`) |

`ruqu-possibility` introduces its own lightweight `CoherenceGate` /
`GateDecision` that operates directly on a field's coherence + entropy (no
external substrate), distinct from the heavyweight `ruqu` min-cut fabric gate
(which requires the optional `ruvector-mincut` feature).

## 8. The possibility-field abstraction (`ruqu-possibility`)

```rust
pub struct Possibility<T> {
    pub id: String,
    pub payload: T,
    pub amplitude: f64,   // magnitude, ≥ 0
    pub phase: f64,       // radians
    pub evidence: Vec<EvidenceReceipt>,
}

pub struct PossibilityField<T> {
    pub candidates: Vec<Possibility<T>>,
    pub collapse_threshold: f64,
}
```

Structural quantities:

- **entropy** — Shannon entropy (bits) of the normalized `amplitude²`
  distribution. `0` when collapsed, `log2(n)` when uniform.
- **coherence** — `|Σ aₖ e^{iφₖ}|² / (Σ aₖ)²` ∈ `[0, 1]`. `1` when phases align
  (constructive); `→0` when phases cancel (contradictory field).
- **field_hash** — BLAKE3 over `(id, amplitude, phase)` per candidate.
- **collapse(seed)** — deterministic, seeded weighted draw → `(Possibility,
  CollapseReceipt)`, fully replayable.

The runtime trait:

```rust
pub trait PossibilityRuntime<I, O> {
    type Error;
    fn construct_field(&self, input: I) -> Result<PossibilityField<O>, Self::Error>;
    fn interfere(&self, field: PossibilityField<O>) -> Result<PossibilityField<O>, Self::Error>;
    fn gate(&self, field: &PossibilityField<O>) -> Result<GateDecision, Self::Error>;
    fn collapse(&self, field: PossibilityField<O>) -> Result<(O, CollapseReceipt), Self::Error>;
}
```

## 9. Worked examples

The original draft listed eight examples; the runtime maps each onto real APIs:

1. **Quantum RAG collapse search** → `ruqu-rag` over
   `ruqu_exotic::interference_search` + `ruqu-possibility`. *(Phase 2, shipped.)*
2. **Reasoning error correction for agents** → `ruqu_exotic::reasoning_qec`,
   surfaced as `VerifierResult`s on the collapse receipt in `ruqu-agent`.
3. **Swarm interference for RuFlo agents** → `ruqu-agent` over
   `ruqu_exotic::swarm_interference`. *(Phase 3, shipped.)*
4. **Coherence gate for autonomous actions** → `ruqu` fabric gate +
   `ruqu-possibility::CoherenceGate` for the field-level decision.
5. **Structural anomaly detection** → `ruqu-sensing` over
   `ruqu_exotic::syndrome_diagnosis`.
6. **Reversible agent memory** → `ruqu_exotic::reversible_memory::ReversibleMemory`,
   audited via `ruqu-receipts`.
7. **Artificial General Engineer search** → `ruqu_algorithms::qaoa` /
   `vqe` design-space exploration (e.g. `run_qaoa(&graph, &QaoaConfig::default())`).
8. **Browser quantum demo layer** → `ruqu-wasm`
   (`wasm-pack build crates/ruqu-wasm --target web`).

## 10. Data model

### 10.1 Possibility candidate

```json
{ "id": "candidate_001", "kind": "retrieval_chunk", "amplitude": 0.72,
  "phase": 0.31, "coherence": 0.84, "source_trust": 0.91,
  "payload_hash": "sha256_value", "evidence": ["receipt_001", "receipt_002"] }
```

### 10.2 Collapse receipt

```json
{ "run_id": "run_...", "field_hash": "blake3_field", "selected_id": "candidate_001",
  "entropy_before": 1.92, "entropy_after": 0.0, "coherence": 0.87,
  "gate_decision": "Permit",
  "rejected": [{ "id": "candidate_014", "reason": "lower interference probability" }] }
```

(The reference implementation hashes with BLAKE3, not SHA-256.)

### 10.3 Agent consensus receipt

```json
{ "swarm_id": "swarm_001", "agents": 7, "consensus": "plan_b",
  "minority_reports": ["plan_c"], "coherence": 0.79,
  "action": "defer_for_human_review" }
```

## 11. Interfaces

- **Rust trait**: `PossibilityRuntime<I, O>` (§8).
- **RuFlo plugin contract**: `ruqu_possibility_runtime` with inputs
  `{task, candidates, policy, risk_signals}` → outputs
  `{selected, receipt, gate: PERMIT|DEFER|DENY}`.
- **MCP tool surface**: `ruqu.construct_field`, `ruqu.interfere`, `ruqu.gate`,
  `ruqu.collapse`, `ruqu.replay`, `ruqu.reverse`.

## 12. Feature flags

Existing `ruqu` directions: `structural`, `decoder`, `attention`, `simd`,
`parallel`, `full`. Proposed AI-oriented umbrella flags (composed at the
workspace / integration layer):

```toml
[features]
ai         = ["ruqu-exotic"]
rag        = ["ai"]
agents     = ["ai"]
receipts   = []
sensing    = []
browser    = ["ruqu-wasm"]
enterprise = ["ai", "rag", "agents", "receipts", "sensing"]
```

## 13. Governance & security

Every high-impact action passes three gates: **evidence** (are sources
sufficient?), **coherence** (is state structurally stable?), and **policy** (is
the action allowed?). Decision mapping: `PERMIT` → execute; `DEFER` → slower
model / more retrieval / human review; `DENY` → block + quarantine + risk
receipt.

Controls: hash all fields before collapse; log selected *and* rejected
candidates; store reversible operations; require deterministic replay for
critical actions; separate simulation output from real-world actuation; require a
policy gate before external side-effects.

Failure modes & mitigations: false coherence → independent verifier + evidence
gate; overconfident collapse → entropy floor + mandatory `DEFER`; bad swarm
convergence → keep minority report + contradiction scan; latency spike →
fallback to standard retrieval / deterministic planner; quantum-branding
confusion → call it *structural possibility runtime* in enterprise material.

## 14. Performance targets

- **Retrieval**: pool 200–1000, 2–5 interference rounds, < 50 ms extra latency,
  +3–8% recall, +10–25% citation faithfulness.
- **Agents**: 3–12 agents, 3–10 rounds, < 2 s consensus (local models),
  −20% contradictions.
- **Coherence gate**: the `ruqu` README reports 468 ns p99 tick, 260 ns avg,
  1,026 ns min-cut query, 3.8 M/s throughput. **Treat as repo-reported baselines
  to verify locally before any external use.**

## 15. Acceptance tests (§22 of the brief)

| # | Test | Where it lives |
|---|------|----------------|
| 1 | Retrieval coherence: interference top-10 faithfulness ≥ +10% over cosine | `ruqu-rag` tests |
| 2 | Reasoning correction: ≥85% detection, <10% false positives | `ruqu-exotic::reasoning_qec` (+ `ruqu-agent`) |
| 3 | Agent swarm: 7 agents / 3 plans → support > 0.75 or `DEFER` | `ruqu-agent` tests |
| 4 | Coherence gate: correlated failures → `DENY`; isolated noise → not `DENY` | `ruqu` fabric / `ruqu-sensing` |
| 5 | Reversible memory: reverse all ops → initial hash | `ruqu-exotic::reversible_memory` |
| 6 | Replay: receipt + seed reproduces selection & coherence within 1e-9 | `ruqu-possibility` + `ruqu-receipts` tests |

## 16. Implementation plan & status

- **Phase 1 — `ruqu-possibility`**: ✅ shipped. Field abstraction, entropy,
  coherence, seeded collapse, gate, receipts, runtime trait, full test suite
  (collapse determinism, replay, receipt hashing, gate behavior).
- **Phase 2 — `ruqu-rag`**: ✅ shipped. Interference reranker + collapse receipts
  + acceptance Test 1 + `quantum_rag_demo` binary (cosine-weak vs
  interference-strong with a printed receipt — the ADR pass condition).
- **Phase 3 — `ruqu-agent`**: ✅ shipped. Swarm collapse consensus + reasoning
  QEC gating + consensus receipts + acceptance Test 3.
- **Sensing — `ruqu-sensing`**: ✅ shipped. Telemetry → syndrome → diagnosis.
- **Governance — `ruqu-receipts`**: ✅ shipped. Hash-chained, replayable logs.
- **Phase 4 — general action gate** & **Phase 5 — browser demo**: deferred to
  follow-up ADRs (the `ruqu` fabric gate and `ruqu-wasm` already provide the
  substrate).

## 17. Non-goals

Do not claim quantum advantage; do not claim production medical use; do not
expose quantum terminology in enterprise UI unless the user opts into expert
mode; do not let probabilistic collapse perform irreversible external actions
without a policy gate; do not replace RuVector or RuFlo — extend them.

## 18. Product names

- Internal: **ruQu Structural Possibility Runtime**
- Public demo: **Possibility OS**
- Enterprise feature: **Coherence-Gated AI**
- RuVector feature: **Interference Reranking** (`ruqu-rag`)
- RuFlo feature: **Swarm Collapse Consensus** (`ruqu-agent`)

## 19. Consequences

- The rUv ecosystem gains a differentiated decision substrate: retrieval with
  coherence, agents with correction, actions with gates, memory with
  reversibility, decisions with receipts.
- New crates are additive and dependency-light; no existing crate's API changes.
- The aspirational/real API gap (§7) is now explicit, so future contributors
  build against the shipped surfaces rather than the illustrative sketches.
- **Pass condition** (build Phase 1 + Phase 2, then demo cosine-weak vs
  ruQu-interference with a collapse receipt) is satisfied by `ruqu-possibility`,
  `ruqu-rag`, and the `quantum_rag_demo` binary.
```
