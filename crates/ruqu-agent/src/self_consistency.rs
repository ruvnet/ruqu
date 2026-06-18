//! # Self-consistency baseline
//!
//! A classical **majority-vote** baseline over the *same* inputs an
//! [`AgentWavefront`](crate::AgentWavefront) carries. This is the real bar the
//! ruqu SOTA landscape report (`docs/research/sota-landscape.md`, recommendation
//! #3) says interference-consensus must clear:
//!
//! > The field aggregates *discretely* (self-consistency +17.9% on GSM8K;
//! > debate; mixture-of-agents). The hard finding: **debate frequently fails to
//! > beat self-consistency at much higher cost**, and **LLMs can't reliably
//! > self-verify**. So interference-consensus must be benchmarked against
//! > **self-consistency** (not single-agent), and reasoning-QEC must use
//! > **external/cross-agent** verification.
//!
//! Self-consistency (Wang et al., 2022, arXiv:2203.11171) samples many reasoning
//! chains and takes a **majority vote** over their final answers. Here each
//! agent's *supporting* vote on a plan stands in for one sampled chain that
//! "answered" that plan; we tally those answers and report the winner.
//!
//! Unlike [`AgentWavefront::coordinate`](crate::AgentWavefront::coordinate),
//! this baseline:
//!
//! - has **no destructive interference** — an opposing vote does not cancel a
//!   supporting one, it simply does not count toward the opposed plan;
//! - has **no coherence gate and no reasoning QEC** — it never defers; it always
//!   names a winner (modulo an exact, unbreakable tie);
//! - is therefore **capturable** by a coordinated bloc of confidently-wrong
//!   agents that out-number the honest agents on a single plan.
//!
//! Everything here is deterministic and allocation-light.

use serde::{Deserialize, Serialize};

use crate::{clamp01, AgentVote, AgentWavefront};

/// The result of a classical majority vote over agent picks.
///
/// `winner` is the plan id with the highest tally, or `None` when there are no
/// effective votes or the top two plans are exactly tied (the baseline does not
/// break ties — see [`SelfConsistency::is_tie`]).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelfConsistency {
    /// The majority-vote winner's plan id, or `None` on no votes / exact tie.
    pub winner: Option<String>,
    /// The winner's share of the total tally in `[0, 1]` (0 when no winner).
    pub vote_share: f64,
    /// Agreement = winner share minus runner-up share, in `[0, 1]`. A proxy for
    /// how decisive the majority is; `0` on a tie or single candidate with no
    /// runner-up is treated as full agreement.
    pub agreement: f64,
    /// All candidate plans with their (possibly weighted) tallies, sorted by
    /// tally descending then plan id ascending for determinism.
    pub tally: Vec<(String, f64)>,
}

impl SelfConsistency {
    /// `true` if no plan received any effective vote.
    pub fn is_empty(&self) -> bool {
        self.tally.is_empty()
    }

    /// `true` if the top two candidates are exactly tied (so `winner` is `None`
    /// despite there being candidates).
    pub fn is_tie(&self) -> bool {
        self.winner.is_none() && !self.tally.is_empty()
    }
}

/// How votes are counted into a self-consistency tally.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VoteWeighting {
    /// Each supporting vote counts as `1.0` (plain majority, the classical
    /// self-consistency formulation).
    Unweighted,
    /// Each supporting vote counts as its `confidence` (confidence-weighted
    /// majority — the higher-fidelity analogue when sampled chains carry a
    /// calibrated confidence).
    ConfidenceWeighted,
}

/// Exact-tie tolerance: tallies within this of each other are treated as tied.
const TIE_EPS: f64 = 1e-9;

/// Compute a self-consistency (majority-vote) result over a set of votes.
///
/// Only **supporting** votes (`support == true`) count toward a plan, mirroring
/// classical self-consistency where each sampled chain casts exactly one vote
/// for the answer it reached — there is no notion of a chain voting *against* an
/// answer. Opposing votes are ignored (this is precisely what makes the baseline
/// capturable, in contrast to interference's destructive cancellation).
///
/// `valid_plan` filters out votes referencing unknown plans so the baseline sees
/// the same plan universe `coordinate` does.
///
/// The result is deterministic: ties in tally are broken by plan id for the sort
/// order, and an exact tie for *first place* yields `winner = None`.
pub fn self_consistency<F>(
    votes: &[AgentVote],
    weighting: VoteWeighting,
    valid_plan: F,
) -> SelfConsistency
where
    F: Fn(&str) -> bool,
{
    // Accumulate per-plan tallies, preserving first-seen order via a parallel
    // index map keyed on plan id. We avoid a HashMap to keep the result order
    // fully deterministic regardless of hasher.
    let mut tally: Vec<(String, f64)> = Vec::new();
    for vote in votes {
        if !vote.support {
            continue;
        }
        if !valid_plan(&vote.plan_id) {
            continue;
        }
        let weight = match weighting {
            VoteWeighting::Unweighted => 1.0,
            VoteWeighting::ConfidenceWeighted => clamp01(vote.confidence),
        };
        if weight <= 0.0 {
            continue;
        }
        match tally.iter_mut().find(|(id, _)| id == &vote.plan_id) {
            Some((_, acc)) => *acc += weight,
            None => tally.push((vote.plan_id.clone(), weight)),
        }
    }

    // Deterministic order: tally desc, then plan id asc.
    tally.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0))
    });

    let total: f64 = tally.iter().map(|(_, w)| w).sum();
    if tally.is_empty() || total <= 0.0 {
        return SelfConsistency {
            winner: None,
            vote_share: 0.0,
            agreement: 0.0,
            tally,
        };
    }

    let top = tally[0].1;
    let runner_up = tally.get(1).map(|(_, w)| *w).unwrap_or(0.0);

    // Exact tie for first place => no winner (the baseline refuses to invent a
    // decision out of a coin flip; the benchmark records this distinctly).
    let tied = (top - runner_up).abs() <= TIE_EPS && tally.len() >= 2;

    let winner = if tied {
        None
    } else {
        Some(tally[0].0.clone())
    };
    let vote_share = if winner.is_some() { top / total } else { 0.0 };
    let agreement = if winner.is_some() {
        ((top - runner_up) / total).clamp(0.0, 1.0)
    } else {
        0.0
    };

    SelfConsistency {
        winner,
        vote_share,
        agreement,
        tally,
    }
}

impl AgentWavefront {
    /// Confidence-weighted self-consistency (majority vote) over this
    /// wavefront's votes, restricted to known plans.
    ///
    /// This is the [report's](crate::self_consistency) recommended *baseline* to
    /// compare [`coordinate`](AgentWavefront::coordinate) against at equal agent
    /// budget. It always names a winner unless there are no effective votes or
    /// the top two plans tie exactly; it never defers and applies no coherence
    /// or reasoning gate.
    pub fn self_consistency(&self) -> SelfConsistency {
        self.self_consistency_with(VoteWeighting::ConfidenceWeighted)
    }

    /// Self-consistency with an explicit [`VoteWeighting`].
    pub fn self_consistency_with(&self, weighting: VoteWeighting) -> SelfConsistency {
        self_consistency(&self.votes, weighting, |id| {
            self.plans.iter().any(|p| p.id == id)
        })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AgentPlan, AgentState};

    fn wf() -> AgentWavefront {
        AgentWavefront::new(0.75)
            .add_agent(AgentState::new("a", "planner", 0.9))
            .add_agent(AgentState::new("b", "critic", 0.9))
            .add_agent(AgentState::new("c", "expert", 0.9))
            .add_plan(AgentPlan::new("A", "plan a", 0.85, vec!["s".into()]))
            .add_plan(AgentPlan::new("B", "plan b", 0.5, vec!["s".into()]))
    }

    #[test]
    fn unweighted_majority_picks_the_most_supported_plan() {
        let w = wf()
            .vote(AgentVote::new("a", "A", 0.6, true))
            .vote(AgentVote::new("b", "A", 0.6, true))
            .vote(AgentVote::new("c", "B", 0.99, true));
        let sc = w.self_consistency_with(VoteWeighting::Unweighted);
        assert_eq!(sc.winner.as_deref(), Some("A"));
        // 2 of 3 votes.
        assert!((sc.vote_share - 2.0 / 3.0).abs() < 1e-12);
        assert!((sc.agreement - 1.0 / 3.0).abs() < 1e-12);
    }

    #[test]
    fn confidence_weighting_can_flip_the_unweighted_winner() {
        // Two weak A votes (0.3 each = 0.6) vs one strong B vote (0.99).
        let w = wf()
            .vote(AgentVote::new("a", "A", 0.3, true))
            .vote(AgentVote::new("b", "A", 0.3, true))
            .vote(AgentVote::new("c", "B", 0.99, true));
        let unweighted = w.self_consistency_with(VoteWeighting::Unweighted);
        let weighted = w.self_consistency_with(VoteWeighting::ConfidenceWeighted);
        assert_eq!(unweighted.winner.as_deref(), Some("A"));
        assert_eq!(weighted.winner.as_deref(), Some("B"));
    }

    #[test]
    fn opposing_votes_do_not_count_against_a_plan() {
        // A: one support. B: one support + one (ignored) oppose.
        let w = wf()
            .vote(AgentVote::new("a", "A", 1.0, true))
            .vote(AgentVote::new("b", "B", 1.0, true))
            .vote(AgentVote::new("c", "B", 1.0, false));
        let sc = w.self_consistency_with(VoteWeighting::Unweighted);
        // The oppose vote is simply not counted -> A and B tie at 1 each.
        assert!(sc.is_tie());
        assert!(sc.winner.is_none());
    }

    #[test]
    fn exact_tie_yields_no_winner() {
        let w = wf()
            .vote(AgentVote::new("a", "A", 1.0, true))
            .vote(AgentVote::new("b", "B", 1.0, true));
        let sc = w.self_consistency_with(VoteWeighting::Unweighted);
        assert!(sc.is_tie());
        assert_eq!(sc.winner, None);
        assert_eq!(sc.vote_share, 0.0);
    }

    #[test]
    fn votes_for_unknown_plans_are_ignored() {
        let w = wf()
            .vote(AgentVote::new("a", "A", 1.0, true))
            .vote(AgentVote::new("b", "ghost", 1.0, true));
        let sc = w.self_consistency_with(VoteWeighting::Unweighted);
        assert_eq!(sc.winner.as_deref(), Some("A"));
        assert!(sc.tally.iter().all(|(id, _)| id != "ghost"));
    }

    #[test]
    fn no_votes_yields_empty() {
        let sc = wf().self_consistency();
        assert!(sc.is_empty());
        assert!(sc.winner.is_none());
    }

    #[test]
    fn deterministic() {
        let w = wf()
            .vote(AgentVote::new("a", "A", 0.6, true))
            .vote(AgentVote::new("b", "A", 0.6, true))
            .vote(AgentVote::new("c", "B", 0.9, true));
        assert_eq!(w.self_consistency(), w.self_consistency());
    }

    #[test]
    fn tally_is_sorted_descending_then_by_id() {
        let w = wf()
            .add_plan(AgentPlan::new("C", "plan c", 0.5, vec!["s".into()]))
            // A and C both get one vote; B gets two.
            .vote(AgentVote::new("a", "C", 1.0, true))
            .vote(AgentVote::new("b", "A", 1.0, true))
            .vote(AgentVote::new("c", "B", 1.0, true))
            .vote(AgentVote::new("a", "B", 1.0, true));
        let sc = w.self_consistency_with(VoteWeighting::Unweighted);
        assert_eq!(sc.tally[0].0, "B"); // highest tally first
                                        // A before C at equal tally (id ascending).
        assert_eq!(sc.tally[1].0, "A");
        assert_eq!(sc.tally[2].0, "C");
    }
}
