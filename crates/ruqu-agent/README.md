# ruqu-agent — Swarm Collapse Consensus

Phase 3 of [ADR-258](../../docs) ("Swarm Collapse Consensus"). Multi-agent
coordination built on quantum-style **interference** instead of classical
voting, with structural reasoning error-correction and auditable receipts.

Agents do not tally votes — they *interfere*. Each agent contributes a complex
amplitude (confidence as magnitude, stance as phase) toward one of several
competing plans. Plans backed by aligned agents are **amplified** (constructive
interference); plans pulled in opposite directions **cancel** (destructive
interference). The surviving interference pattern is lifted into a possibility
field, gated for structural coherence, and the winning plan's reasoning chain is
run through reasoning QEC before the swarm is allowed to act.

## The consensus model

| Concept | Type | Role |
|---|---|---|
| `AgentState` | agent + role + confidence | Participant metadata (planner, critic, security, cost, latency, governance, expert…). |
| `AgentPlan` | id + description + `evidence_support` + `steps` | A candidate plan; `evidence_support ∈ [0,1]` is checked against the consensus threshold. |
| `AgentVote` | agent → plan, confidence, support | A stance: `support=true` ⇒ phase 0 (constructive), `support=false` ⇒ phase π (destructive). |
| `AgentWavefront` | agents + plans + votes + threshold | The full coordination round; builder API + `coordinate(seed)`. |
| `ConsensusOutcome` | plan id + minority reports + coherence + gate + action + receipt | The result, JSON-serializable. |

### Pipeline (`AgentWavefront::coordinate(seed)`)

```text
votes ──▶ SwarmInterference.decide()  interference-ranked plans (|Σ amplitudes|²)
      ──▶ PossibilityField<AgentPlan>  amplitude = √probability,
                                       phase = 0 (net-supported) / π (net-opposed)
      ──▶ CoherenceGate.evaluate()     PERMIT / DEFER / DENY
      ──▶ ReasoningTrace.run_qec()     structural reasoning integrity on the winner
      ──▶ decision: execute | defer_for_human_review
      ──▶ CollapseReceipt (+ reasoning-QEC VerifierResult)
```

1. **Interference.** Every vote becomes an `AgentContribution`. `decide()`
   returns each plan's post-interference probability and its constructive vs
   destructive contributor counts. A near-tie between the top two plans is
   flagged as a deadlock.
2. **Possibility field.** Each ranked plan becomes a `Possibility` with
   amplitude `√probability` and phase `0` (net-supported) or `π` (net-opposed).
   The field's phase **coherence** measures how aligned the surviving plans are.
3. **Coherence gate.** The default `CoherenceGate` maps coherence/entropy to
   `Permit`, `Defer`, or `Deny`.
4. **Reasoning QEC.** The winning plan's `steps` are encoded as a 1-D
   repetition code (clamped to ≤ 13 steps for the 25-qubit budget), noise is
   injected, and syndrome extraction detects reasoning incoherence. The result
   is folded into a `VerifierResult` named `reasoning_qec`.
5. **Collapse + receipt.** The field is collapsed deterministically for `seed`,
   producing a `CollapseReceipt`; the reasoning-QEC verifier is appended to it.

## Decision mapping (ADR §22 Test 3)

The swarm returns `action = "execute"` **only when all three hold**:

1. the winning plan's `evidence_support` **>** `consensus_threshold` (e.g. 0.75),
2. the coherence gate returns `GateDecision::Permit`, **and**
3. reasoning QEC passes — the trace is *decodable* (any detected reasoning
   errors are structurally correctable), and the round is not deadlocked.

If any condition fails, `action = "defer_for_human_review"` and
`consensus_plan_id` is `None`. **A low-support plan can never be silently
executed**, and a contradictory/deadlocked field defers rather than guessing.

## Determinism

`coordinate(seed)` is fully deterministic: the same wavefront and seed always
produce the same action, the same selected plan, and the same receipt hash.
Interference math, the seeded collapse draw, and the seeded reasoning-QEC noise
are all driven from the supplied seed.

## Example

```rust
use ruqu_agent::{AgentWavefront, AgentState, AgentPlan, AgentVote};

let wf = AgentWavefront::new(0.75)
    .add_agent(AgentState::new("planner", "planner", 0.9))
    .add_agent(AgentState::new("critic", "critic", 0.9))
    .add_plan(AgentPlan::new(
        "A", "well-evidenced rollout", 0.85,
        vec!["gather evidence".into(), "stage rollout".into()],
    ))
    .vote(AgentVote::new("planner", "A", 0.95, true))
    .vote(AgentVote::new("critic", "A", 0.9, true));

let outcome = wf.coordinate(0xC0FFEE).unwrap();
println!("{}", outcome.to_json());
assert!(outcome.action == "execute" || outcome.action == "defer_for_human_review");
```

## Tests

`cargo test -p ruqu-agent` covers ADR §22 Test 3:

- 7 agents / 3 competing plans → converges to a `> 0.75`-support plan (execute)
  or defers; never silently executes a weak plan, plus an explicit guard that
  the strong-consensus case reaches the `execute` path.
- no plan over the threshold → `defer_for_human_review`.
- the receipt carries a `reasoning_qec` `VerifierResult` and serializes to JSON.
- two equally-supported, opposed plans → low coherence / deadlock → defer.
- determinism, ignored votes for unknown plans, and the empty-wavefront error.
```
