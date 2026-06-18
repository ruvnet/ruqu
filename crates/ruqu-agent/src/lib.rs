//! # ruqu-agent — Swarm Collapse Consensus
//!
//! Phase 3 of ADR-258. Multi-agent coordination where agents do **not** vote in
//! the classical sense — they *interfere*. Each agent contributes a complex
//! amplitude (confidence + stance) toward one of several competing plans.
//! Reinforcing agents amplify a plan (constructive interference); conflicting
//! agents cancel it (destructive interference). The surviving interference
//! pattern is lifted into a [`PossibilityField`] of plans, gated for structural
//! coherence, and the winning plan's reasoning chain is run through reasoning
//! QEC. Only when evidence, coherence, *and* reasoning integrity all agree does
//! the swarm `execute`; otherwise it defers for human review.
//!
//! ## Pipeline
//!
//! ```text
//!   votes ──▶ SwarmInterference.decide()  (interference-ranked plans)
//!         ──▶ PossibilityField<AgentPlan>  (amplitude = √probability,
//!                                            phase = 0 supported / π opposed)
//!         ──▶ CoherenceGate.evaluate()     (PERMIT / DEFER / DENY)
//!         ──▶ ReasoningTrace.run_qec()     (structural reasoning integrity)
//!         ──▶ decision: execute | defer_for_human_review
//!         ──▶ CollapseReceipt (+ reasoning-QEC VerifierResult)
//! ```
//!
//! ## Decision mapping (ADR §22 Test 3)
//!
//! The swarm `execute`s a plan only if **all** of the following hold:
//!
//! 1. the winning plan's `evidence_support` exceeds the consensus threshold,
//! 2. the coherence gate returns [`GateDecision::Permit`], and
//! 3. reasoning QEC on the plan's steps passes (the trace is decodable, i.e.
//!    any detected reasoning errors are structurally correctable).
//!
//! Any failure yields `"defer_for_human_review"`. A low-support plan can never be
//! silently executed.
//!
//! Everything is deterministic for a given seed.

use anyhow::Result;
use serde::{Deserialize, Serialize};

use ruqu_exotic::reasoning_qec::{ReasoningQecConfig, ReasoningStep, ReasoningTrace};
use ruqu_exotic::swarm_interference::{
    Action, AgentContribution, SwarmDecision, SwarmInterference,
};
use ruqu_possibility::{
    CoherenceGate, CollapseReceipt, GateDecision, Possibility, PossibilityField, VerifierResult,
};

use std::f64::consts::PI;

pub mod self_consistency;
pub use self_consistency::{self_consistency, SelfConsistency, VoteWeighting};

/// Maximum reasoning steps that fit the reasoning-QEC qubit budget
/// (`2·num_steps − 1 ≤ 25`).
const MAX_QEC_STEPS: usize = 13;

/// Noise rate used when probing the winning plan's reasoning chain.
const QEC_NOISE_RATE: f64 = 0.05;

/// The two terminal actions the swarm can take.
const ACTION_EXECUTE: &str = "execute";
const ACTION_DEFER: &str = "defer_for_human_review";

// ---------------------------------------------------------------------------
// Domain types
// ---------------------------------------------------------------------------

/// A participating agent and the role it plays in the swarm.
///
/// Roles are advisory metadata (planner, critic, security, cost, latency,
/// governance, expert…); the interference math treats all agents uniformly,
/// distinguishing them only by their per-vote confidence and stance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentState {
    /// Stable agent identifier.
    pub id: String,
    /// The agent's role (e.g. `planner`, `critic`, `security`).
    pub role: String,
    /// The agent's baseline confidence in `[0, 1]`.
    pub confidence: f64,
}

impl AgentState {
    /// Create an agent state. `confidence` is clamped to `[0, 1]`.
    pub fn new(id: impl Into<String>, role: impl Into<String>, confidence: f64) -> Self {
        Self {
            id: id.into(),
            role: role.into(),
            confidence: clamp01(confidence),
        }
    }
}

/// A candidate plan the swarm can converge on.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentPlan {
    /// Stable plan identifier (used as the interference [`Action`] id).
    pub id: String,
    /// Human-readable description of the plan.
    pub description: String,
    /// Aggregate evidence support for the plan in `[0, 1]`. This is the
    /// quantity compared against the consensus threshold before executing.
    pub evidence_support: f64,
    /// Ordered reasoning steps — fed through reasoning QEC for the winner.
    pub steps: Vec<String>,
}

impl AgentPlan {
    /// Create a plan. `evidence_support` is clamped to `[0, 1]`.
    pub fn new(
        id: impl Into<String>,
        description: impl Into<String>,
        evidence_support: f64,
        steps: Vec<String>,
    ) -> Self {
        Self {
            id: id.into(),
            description: description.into(),
            evidence_support: clamp01(evidence_support),
            steps,
        }
    }
}

/// A single agent's stance on a single plan.
///
/// `support = true` contributes constructively (phase 0); `support = false`
/// contributes destructively (phase π), cancelling supporters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentVote {
    /// The voting agent's id.
    pub agent_id: String,
    /// The plan the vote concerns.
    pub plan_id: String,
    /// Confidence (magnitude) of the vote in `[0, 1]`.
    pub confidence: f64,
    /// `true` to support, `false` to oppose.
    pub support: bool,
}

impl AgentVote {
    /// Create a vote. `confidence` is clamped to `[0, 1]`.
    pub fn new(
        agent_id: impl Into<String>,
        plan_id: impl Into<String>,
        confidence: f64,
        support: bool,
    ) -> Self {
        Self {
            agent_id: agent_id.into(),
            plan_id: plan_id.into(),
            confidence: clamp01(confidence),
            support,
        }
    }
}

/// The full state of a coordination round: the agents, the competing plans, the
/// votes cast, and the consensus threshold the winner must clear to execute.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentWavefront {
    /// Participating agents.
    pub agents: Vec<AgentState>,
    /// Competing plans.
    pub plans: Vec<AgentPlan>,
    /// Votes cast by agents on plans.
    pub votes: Vec<AgentVote>,
    /// Evidence-support threshold a plan must exceed to be executed.
    pub consensus_threshold: f64,
}

impl AgentWavefront {
    /// Create an empty wavefront with the given consensus threshold (clamped to
    /// `[0, 1]`).
    pub fn new(consensus_threshold: f64) -> Self {
        Self {
            agents: Vec::new(),
            plans: Vec::new(),
            votes: Vec::new(),
            consensus_threshold: clamp01(consensus_threshold),
        }
    }

    /// Register an agent (builder style).
    pub fn add_agent(mut self, agent: AgentState) -> Self {
        self.agents.push(agent);
        self
    }

    /// Register a candidate plan (builder style).
    pub fn add_plan(mut self, plan: AgentPlan) -> Self {
        self.plans.push(plan);
        self
    }

    /// Cast a vote (builder style).
    pub fn vote(mut self, vote: AgentVote) -> Self {
        self.votes.push(vote);
        self
    }

    /// Look up a plan by id.
    fn plan(&self, id: &str) -> Option<&AgentPlan> {
        self.plans.iter().find(|p| p.id == id)
    }

    /// Run the swarm-collapse consensus pipeline deterministically for `seed`.
    ///
    /// See the [crate-level docs](crate) for the pipeline and decision mapping.
    pub fn coordinate(&self, seed: u64) -> Result<ConsensusOutcome> {
        // --- 1. Interference: turn votes into a ranked set of plan decisions. --
        let mut swarm = SwarmInterference::new();
        for vote in &self.votes {
            // Only votes that reference a known plan contribute.
            let Some(plan) = self.plan(&vote.plan_id) else {
                continue;
            };
            let action = Action {
                id: plan.id.clone(),
                description: plan.description.clone(),
            };
            swarm.contribute(AgentContribution::new(
                &vote.agent_id,
                action,
                vote.confidence,
                vote.support,
            ));
        }

        let decisions = swarm.decide();
        if decisions.is_empty() {
            return Err(anyhow::anyhow!(
                "no interference decisions: the wavefront has no votes referencing known plans"
            ));
        }

        // A near-tie between the top two plans is a deadlock / contradiction.
        let deadlocked = swarm.is_deadlocked(1e-6);

        // --- 2. Lift the interference pattern into a possibility field. -------
        // amplitude = √probability (so probability = amplitude²);
        // phase     = 0 for net-supported plans, π for net-opposed plans.
        let candidates: Vec<Possibility<AgentPlan>> = decisions
            .iter()
            .filter_map(|d| self.plan(&d.action.id).map(|plan| (d, plan)))
            .map(|(d, plan)| {
                let amplitude = d.probability.max(0.0).sqrt();
                let phase = decision_phase(d);
                Possibility::new(plan.id.clone(), plan.clone(), amplitude, phase)
            })
            .collect();

        let field = PossibilityField::new(candidates);
        let coherence = field.coherence();
        let gate = CoherenceGate::with_defaults().evaluate(&field);

        // --- 3. Select the winning plan (top interference probability). ------
        let winner_decision = &decisions[0];
        let winner = self
            .plan(&winner_decision.action.id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("winning decision references an unknown plan"))?;

        // Minority reports: every other plan that received any vote.
        let minority_reports: Vec<String> = decisions
            .iter()
            .skip(1)
            .map(|d| d.action.id.clone())
            .collect();

        // --- 4. Reasoning QEC on the winner's reasoning chain. ---------------
        let qec_verifier = run_reasoning_qec(&winner, seed)?;
        let reasoning_ok = qec_verifier.passed;

        // --- 5. Collapse the field for an auditable receipt. -----------------
        let (_selected, mut receipt) = field.collapse(seed)?;
        // Attach the reasoning-QEC verifier result to the receipt.
        receipt.verifier_results.push(qec_verifier);

        // --- 6. Decision mapping. -------------------------------------------
        let evidence_ok = winner.evidence_support > self.consensus_threshold;
        let gate_permits = gate.is_permit();

        let (action, consensus_plan_id) =
            if evidence_ok && gate_permits && reasoning_ok && !deadlocked {
                (ACTION_EXECUTE.to_string(), Some(winner.id.clone()))
            } else {
                (ACTION_DEFER.to_string(), None)
            };

        Ok(ConsensusOutcome {
            consensus_plan_id,
            minority_reports,
            coherence,
            gate,
            action,
            receipt,
        })
    }
}

// ---------------------------------------------------------------------------
// Outcome
// ---------------------------------------------------------------------------

/// The result of a [`AgentWavefront::coordinate`] round.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConsensusOutcome {
    /// The agreed plan id when the swarm `execute`s; `None` when deferring.
    pub consensus_plan_id: Option<String>,
    /// The ids of plans that lost the interference ranking.
    pub minority_reports: Vec<String>,
    /// Phase coherence of the plan field at decision time, in `[0, 1]`.
    pub coherence: f64,
    /// The coherence-gate decision over the plan field.
    pub gate: GateDecision,
    /// `"execute"` or `"defer_for_human_review"`.
    pub action: String,
    /// The auditable collapse receipt, carrying the reasoning-QEC verifier.
    pub receipt: CollapseReceipt,
}

impl ConsensusOutcome {
    /// `true` if the swarm decided to execute.
    pub fn executed(&self) -> bool {
        self.action == ACTION_EXECUTE
    }

    /// `true` if the swarm deferred for human review.
    pub fn deferred(&self) -> bool {
        self.action == ACTION_DEFER
    }

    /// Pretty-printed JSON encoding of the outcome.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Clamp a value into `[0, 1]`, mapping non-finite values to 0.
pub(crate) fn clamp01(x: f64) -> f64 {
    if x.is_finite() {
        x.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// Phase for a plan in the possibility field: `0` when net-supported (more
/// constructive than destructive contributors), `π` when net-opposed.
fn decision_phase(d: &SwarmDecision) -> f64 {
    if d.constructive_count >= d.destructive_count {
        0.0
    } else {
        PI
    }
}

/// Run reasoning QEC over a plan's reasoning steps and fold the result into a
/// [`VerifierResult`].
///
/// The chain is clamped to [`MAX_QEC_STEPS`] to stay within the qubit budget.
/// Step confidence is derived from the plan's `evidence_support`. The verifier
/// passes when the trace is **decodable** — i.e. any errors injected into the
/// reasoning chain are structurally correctable (the defining guarantee of QEC).
/// An undecodable trace (too many simultaneous errors to correct) fails.
fn run_reasoning_qec(plan: &AgentPlan, seed: u64) -> Result<VerifierResult> {
    // Reasoning QEC needs at least one step; synthesize one if the plan is bare.
    let labels: Vec<String> = if plan.steps.is_empty() {
        vec![format!("{}::implicit", plan.id)]
    } else {
        plan.steps.iter().take(MAX_QEC_STEPS).cloned().collect()
    };

    let confidence = clamp01(plan.evidence_support);
    let steps: Vec<ReasoningStep> = labels
        .iter()
        .map(|label| ReasoningStep {
            label: label.clone(),
            confidence,
        })
        .collect();
    let num_steps = steps.len();

    let config = ReasoningQecConfig {
        num_steps,
        noise_rate: QEC_NOISE_RATE,
        seed: Some(seed),
    };

    let mut trace = ReasoningTrace::new(steps, config)
        .map_err(|e| anyhow::anyhow!("failed to build reasoning trace: {e}"))?;
    let result = trace
        .run_qec()
        .map_err(|e| anyhow::anyhow!("reasoning QEC failed: {e}"))?;

    // QEC passes when the trace is decodable: detected errors are correctable.
    let passed = result.is_decodable;
    let detail = format!(
        "reasoning QEC over {} step(s): decodable={}, error_steps={:?}, corrected_fidelity={:.4}",
        result.num_steps, result.is_decodable, result.error_steps, result.corrected_fidelity
    );

    Ok(VerifierResult::new("reasoning_qec", passed, detail))
}

// ---------------------------------------------------------------------------
// Tests — ADR §22 Test 3
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const SEED: u64 = 0xC0FFEE;

    /// Seven agents, three competing plans:
    /// - A: strong, well-supported (evidence_support ≈ 0.85), broad agent support
    /// - B: medium support
    /// - C: weak / contradicted
    ///
    /// Build a wavefront where A clearly dominates the interference pattern.
    fn seven_agent_wavefront() -> AgentWavefront {
        let roles = [
            "planner",
            "critic",
            "security",
            "cost",
            "latency",
            "governance",
            "expert",
        ];
        let mut wf = AgentWavefront::new(0.75);
        for (i, role) in roles.iter().enumerate() {
            wf = wf.add_agent(AgentState::new(format!("agent_{i}"), *role, 0.9));
        }

        wf = wf
            .add_plan(AgentPlan::new(
                "A",
                "ship the well-evidenced rollout",
                0.85,
                vec![
                    "gather evidence".into(),
                    "review risk".into(),
                    "stage rollout".into(),
                    "monitor".into(),
                ],
            ))
            .add_plan(AgentPlan::new(
                "B",
                "the middling alternative",
                0.55,
                vec!["draft".into(), "partial review".into()],
            ))
            .add_plan(AgentPlan::new(
                "C",
                "the contradicted long shot",
                0.2,
                vec!["wing it".into()],
            ));

        // Plan A: 6 strong supporters, 1 mild opposer -> strong net amplitude.
        for i in 0..6 {
            wf = wf.vote(AgentVote::new(format!("agent_{i}"), "A", 0.95, true));
        }
        wf = wf.vote(AgentVote::new("agent_6", "A", 0.3, false));

        // Plan B: 3 supporters, 2 opposers -> partly cancelled.
        for i in 0..3 {
            wf = wf.vote(AgentVote::new(format!("agent_{i}"), "B", 0.6, true));
        }
        for i in 3..5 {
            wf = wf.vote(AgentVote::new(format!("agent_{i}"), "B", 0.6, false));
        }

        // Plan C: 1 supporter, 3 opposers -> net-opposed.
        wf = wf.vote(AgentVote::new("agent_0", "C", 0.5, true));
        for i in 1..4 {
            wf = wf.vote(AgentVote::new(format!("agent_{i}"), "C", 0.7, false));
        }

        wf
    }

    #[test]
    fn converges_to_well_supported_plan_or_defers() {
        let wf = seven_agent_wavefront();
        let outcome = wf.coordinate(SEED).unwrap();

        // It must never silently execute a low-support plan.
        if outcome.executed() {
            let plan_id = outcome.consensus_plan_id.as_deref().unwrap();
            let plan = wf.plan(plan_id).unwrap();
            assert!(
                plan.evidence_support > 0.75,
                "executed plan {plan_id} has support {} <= 0.75",
                plan.evidence_support
            );
            // The dominant plan should be A.
            assert_eq!(plan_id, "A");
        } else {
            assert_eq!(outcome.action, "defer_for_human_review");
        }
    }

    #[test]
    fn strong_consensus_reaches_execute_path() {
        // Guards against the converges test passing vacuously via the defer
        // branch: a strongly-supported, broadly-backed plan must execute.
        let wf = seven_agent_wavefront();
        let outcome = wf.coordinate(SEED).unwrap();
        assert!(
            outcome.executed(),
            "expected execute, got {} (gate={:?}, coherence={})",
            outcome.action,
            outcome.gate,
            outcome.coherence
        );
        assert_eq!(outcome.consensus_plan_id.as_deref(), Some("A"));
        assert!(outcome.gate.is_permit());
    }

    #[test]
    fn coordinate_is_deterministic() {
        let wf = seven_agent_wavefront();
        let a = wf.coordinate(SEED).unwrap();
        let b = wf.coordinate(SEED).unwrap();
        assert_eq!(a.action, b.action);
        assert_eq!(a.consensus_plan_id, b.consensus_plan_id);
        assert_eq!(a.receipt.receipt_hash(), b.receipt.receipt_hash());
        assert!((a.coherence - b.coherence).abs() < 1e-12);
    }

    #[test]
    fn no_plan_over_threshold_defers() {
        // Every plan sits at or below the 0.75 threshold.
        let wf = AgentWavefront::new(0.75)
            .add_agent(AgentState::new("a", "planner", 0.7))
            .add_agent(AgentState::new("b", "critic", 0.7))
            .add_agent(AgentState::new("c", "expert", 0.7))
            .add_plan(AgentPlan::new(
                "P",
                "weakly evidenced plan",
                0.6,
                vec!["s1".into(), "s2".into()],
            ))
            .add_plan(AgentPlan::new(
                "Q",
                "another weak plan",
                0.5,
                vec!["s1".into()],
            ))
            .vote(AgentVote::new("a", "P", 0.9, true))
            .vote(AgentVote::new("b", "P", 0.9, true))
            .vote(AgentVote::new("c", "Q", 0.4, true));

        let outcome = wf.coordinate(SEED).unwrap();
        assert_eq!(outcome.action, "defer_for_human_review");
        assert!(outcome.consensus_plan_id.is_none());
    }

    #[test]
    fn receipt_carries_reasoning_qec_and_serializes() {
        let wf = seven_agent_wavefront();
        let outcome = wf.coordinate(SEED).unwrap();

        // The reasoning-QEC verifier must be present on the receipt.
        let has_qec = outcome
            .receipt
            .verifier_results
            .iter()
            .any(|v| v.name == "reasoning_qec");
        assert!(has_qec, "receipt is missing the reasoning_qec verifier");

        // The outcome and its receipt both serialize to JSON.
        let outcome_json = outcome.to_json();
        assert!(outcome_json.contains("reasoning_qec"));
        assert!(outcome_json.contains("\"action\""));

        let receipt_json = outcome.receipt.to_json();
        assert!(receipt_json.contains("reasoning_qec"));

        // Round-trip the outcome through JSON.
        let parsed: ConsensusOutcome = serde_json::from_str(&outcome_json).unwrap();
        assert_eq!(parsed.action, outcome.action);
    }

    #[test]
    fn deadlocked_opposed_plans_defer() {
        // Two plans with exactly equal, opposed support -> deadlock / low coherence.
        let wf = AgentWavefront::new(0.75)
            .add_agent(AgentState::new("a", "planner", 0.9))
            .add_agent(AgentState::new("b", "critic", 0.9))
            .add_agent(AgentState::new("c", "security", 0.9))
            .add_agent(AgentState::new("d", "governance", 0.9))
            .add_plan(AgentPlan::new(
                "X",
                "fork one",
                0.85,
                vec!["s1".into(), "s2".into()],
            ))
            .add_plan(AgentPlan::new(
                "Y",
                "fork two",
                0.85,
                vec!["s1".into(), "s2".into()],
            ))
            // X: one for, one against -> cancels.
            .vote(AgentVote::new("a", "X", 1.0, true))
            .vote(AgentVote::new("b", "X", 1.0, false))
            // Y: one for, one against -> cancels.
            .vote(AgentVote::new("c", "Y", 1.0, true))
            .vote(AgentVote::new("d", "Y", 1.0, false));

        let outcome = wf.coordinate(SEED).unwrap();
        // Both plans fully cancel: equal (zero) probability => deadlock.
        assert_eq!(outcome.action, "defer_for_human_review");
        // Coherence of a contradictory field should be low.
        assert!(
            outcome.coherence < 0.7,
            "expected low coherence, got {}",
            outcome.coherence
        );
    }

    #[test]
    fn votes_for_unknown_plans_are_ignored() {
        let wf = AgentWavefront::new(0.75)
            .add_agent(AgentState::new("a", "planner", 0.9))
            .add_plan(AgentPlan::new("A", "real plan", 0.9, vec!["s1".into()]))
            .vote(AgentVote::new("a", "A", 0.95, true))
            .vote(AgentVote::new("a", "ghost", 0.95, true));

        let outcome = wf.coordinate(SEED).unwrap();
        // Only plan A exists in the field; no ghost plan leaks through.
        assert!(outcome.minority_reports.iter().all(|id| id != "ghost"));
        // A single strong, well-supported plan should execute.
        assert_eq!(outcome.action, "execute");
        assert_eq!(outcome.consensus_plan_id.as_deref(), Some("A"));
    }

    #[test]
    fn empty_wavefront_errors() {
        let wf = AgentWavefront::new(0.75);
        assert!(wf.coordinate(SEED).is_err());
    }
}
