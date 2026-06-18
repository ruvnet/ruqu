//! # Honest head-to-head: interference-consensus vs self-consistency
//!
//! Implements recommendation #3 of the ruqu SOTA landscape report
//! (`docs/research/sota-landscape.md`):
//!
//! > Multi-agent **debate / consensus** methods frequently FAIL to beat plain
//! > **self-consistency** (majority vote over sampled chains) at much higher
//! > cost. So ruqu's interference-based "swarm consensus" MUST be benchmarked
//! > against self-consistency, not just single-agent.
//!
//! This harness defines a synthetic task family with a **known ground-truth best
//! plan** and parameterized difficulty, then compares:
//!
//! - **interference** — [`AgentWavefront::coordinate`], the full pipeline
//!   (interference ranking + coherence gate + reasoning QEC + defer), and
//! - **self-consistency** — [`AgentWavefront::self_consistency`], a classical
//!   confidence-weighted majority vote over the *same* agents/votes.
//!
//! Both methods see the **same agent budget** (the same votes), so cost is held
//! equal and only the aggregation rule differs.
//!
//! ## Regimes
//!
//! - **(a) honest majority** — most agents support the correct plan. Both
//!   methods should land on the correct plan. We assert **parity** (no claimed
//!   interference win here) per the report's "don't fabricate a win" guidance.
//! - **(b) colluding bloc** — a coordinated bloc of confidently-wrong agents all
//!   push one contradicted plan, slightly out-numbering the honest agents who
//!   support the correct plan *and oppose the bloc's plan*. Naive majority can be
//!   **captured** (it ignores opposition). Interference's destructive
//!   cancellation + coherence gate should resist capture — either by ranking the
//!   bad plan down or by **deferring** (the safe action). We assert interference
//!   has a strictly lower **capture rate**.
//! - **(c) deadlock / near-tie** — two strong plans are pulled to a near-tie.
//!   Interference should **defer**; naive majority must pick one arbitrarily and
//!   is right only ~half the time. We assert interference's defer rate is high
//!   and that it is not "captured" into a confident wrong answer.
//!
//! Each regime runs over many seeds; we aggregate accuracy / defer-rate /
//! capture-rate per method and print a results table.

use ruqu_agent::{AgentPlan, AgentState, AgentVote, AgentWavefront};

/// Number of independent seeds per regime.
const SEEDS: u64 = 200;

/// Deterministic small PRNG (SplitMix64) so the benchmark needs no rand dep here
/// and is fully reproducible.
struct SplitMix64(u64);
impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self(seed)
    }
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// Uniform in [0, 1).
    fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
    /// Jitter a base confidence by ±`spread`, clamped to [0.05, 0.999].
    fn jitter(&mut self, base: f64, spread: f64) -> f64 {
        let d = (self.unit() * 2.0 - 1.0) * spread;
        (base + d).clamp(0.05, 0.999)
    }
}

/// The known ground-truth correct plan id for every task in this family.
const CORRECT: &str = "CORRECT";
/// The plan a colluding bloc pushes (always wrong).
const BAD: &str = "BAD";
/// A neutral third option.
const OTHER: &str = "OTHER";

/// Per-method outcome on one task instance.
#[derive(Clone, Copy)]
enum Pick {
    /// Method named this as the consensus plan.
    Plan(&'static str),
    /// Method declined to decide (defer / tie).
    Abstain,
}

/// Tally for one method across many seeds.
#[derive(Default, Clone)]
struct Stats {
    n: u64,
    correct: u64,
    /// Named a wrong, confident plan.
    wrong: u64,
    /// Declined (defer / tie).
    abstain: u64,
    /// Specifically named the colluding bloc's BAD plan (capture).
    captured: u64,
}
impl Stats {
    fn record(&mut self, pick: Pick) {
        self.n += 1;
        match pick {
            Pick::Plan(id) => {
                if id == CORRECT {
                    self.correct += 1;
                } else {
                    self.wrong += 1;
                    if id == BAD {
                        self.captured += 1;
                    }
                }
            }
            Pick::Abstain => self.abstain += 1,
        }
    }
    fn accuracy(&self) -> f64 {
        self.correct as f64 / self.n as f64
    }
    fn defer_rate(&self) -> f64 {
        self.abstain as f64 / self.n as f64
    }
    fn capture_rate(&self) -> f64 {
        self.captured as f64 / self.n as f64
    }
}

/// Map the interference pipeline outcome onto a [`Pick`].
fn interference_pick(wf: &AgentWavefront, seed: u64) -> Pick {
    match wf.coordinate(seed) {
        Ok(o) if o.executed() => {
            // Leak the executed plan id as a &'static str by matching the known set.
            match o.consensus_plan_id.as_deref() {
                Some(CORRECT) => Pick::Plan(CORRECT),
                Some(BAD) => Pick::Plan(BAD),
                Some(OTHER) => Pick::Plan(OTHER),
                _ => Pick::Plan(OTHER),
            }
        }
        // Defer or pipeline error both count as "declined to act".
        _ => Pick::Abstain,
    }
}

/// Map the self-consistency baseline onto a [`Pick`].
fn self_consistency_pick(wf: &AgentWavefront) -> Pick {
    let sc = wf.self_consistency();
    match sc.winner.as_deref() {
        Some(CORRECT) => Pick::Plan(CORRECT),
        Some(BAD) => Pick::Plan(BAD),
        Some(OTHER) => Pick::Plan(OTHER),
        Some(_) => Pick::Abstain,
        None => Pick::Abstain, // exact tie / no votes
    }
}

/// Three plans shared by every regime. Evidence support is set so the CORRECT
/// plan clears the 0.75 threshold while BAD/OTHER do not (the gate must never
/// silently execute a weakly-evidenced plan).
fn base_wavefront() -> AgentWavefront {
    AgentWavefront::new(0.75)
        .add_plan(AgentPlan::new(
            CORRECT,
            "the ground-truth best plan",
            0.88,
            vec![
                "gather evidence".into(),
                "review risk".into(),
                "stage rollout".into(),
                "monitor".into(),
            ],
        ))
        .add_plan(AgentPlan::new(
            BAD,
            "the contradicted plan a bloc pushes",
            0.30,
            vec!["assert".into(), "ship".into()],
        ))
        .add_plan(AgentPlan::new(
            OTHER,
            "a neutral third option",
            0.50,
            vec!["draft".into(), "wait".into()],
        ))
}

/// (a) Honest majority: most agents support CORRECT, with light noise. No
/// colluding bloc. Both methods should be correct.
fn regime_honest(seed: u64) -> AgentWavefront {
    let mut r = SplitMix64::new(seed ^ 0xA11);
    let mut wf = base_wavefront();
    let n = 7;
    for i in 0..n {
        wf = wf.add_agent(AgentState::new(format!("agent_{i}"), "expert", 0.9));
    }
    // 6 honest supporters of CORRECT, 1 dissenter who supports OTHER.
    for i in 0..6 {
        wf = wf.vote(AgentVote::new(
            format!("agent_{i}"),
            CORRECT,
            r.jitter(0.9, 0.08),
            true,
        ));
    }
    wf = wf.vote(AgentVote::new("agent_6", OTHER, r.jitter(0.6, 0.1), true));
    wf
}

/// (b) Colluding bloc: a bloc of confidently-wrong agents all support BAD and
/// out-numbers the honest CORRECT supporters by one. Crucially, the honest
/// agents *also oppose* BAD (they recognize the contradiction). Naive majority
/// counts only the (larger) supporting bloc and is captured; interference lets
/// the honest opposition destructively cancel BAD.
fn regime_colluding(seed: u64) -> AgentWavefront {
    let mut r = SplitMix64::new(seed ^ 0xB10C);
    let mut wf = base_wavefront();
    let honest = 4usize; // support CORRECT, oppose BAD
    let bloc = 5usize; // confidently support BAD (one more than honest)
    let total = honest + bloc;
    for i in 0..total {
        wf = wf.add_agent(AgentState::new(format!("agent_{i}"), "expert", 0.9));
    }
    // Honest agents: strongly support CORRECT and strongly oppose BAD.
    for i in 0..honest {
        let id = format!("agent_{i}");
        wf = wf.vote(AgentVote::new(&id, CORRECT, r.jitter(0.9, 0.06), true));
        wf = wf.vote(AgentVote::new(&id, BAD, r.jitter(0.9, 0.06), false));
    }
    // Colluding bloc: confidently support BAD (and nothing else).
    for i in honest..total {
        let id = format!("agent_{i}");
        wf = wf.vote(AgentVote::new(&id, BAD, r.jitter(0.95, 0.04), true));
    }
    wf
}

/// (c) Deadlock / near-tie: two equal-sized blocs back CORRECT and OTHER, and
/// each plan is *also opposed* by an equal-confidence agent from the rival bloc,
/// so every plan's supporting and opposing amplitudes nearly cancel. The seed
/// only chooses which plan is marginally ahead, keeping the two top plans within
/// a hair of each other. Interference should detect the deadlock / low coherence
/// and DEFER; naive majority must pick one of the (tied) plans arbitrarily and
/// is correct only about half the time.
fn regime_deadlock(seed: u64) -> AgentWavefront {
    let mut r = SplitMix64::new(seed ^ 0xDEAD);
    let mut wf = base_wavefront();
    let per = 3usize;
    let total = per * 2;
    for i in 0..total {
        wf = wf.add_agent(AgentState::new(format!("agent_{i}"), "expert", 0.9));
    }
    // A single shared, near-1.0 confidence keeps support and opposition matched
    // so each plan's net amplitude collapses toward zero (a true contradiction).
    // Tiny per-seed jitter (<= 1e-3) decides which plan is infinitesimally ahead
    // without breaking the near-tie.
    let conf = 0.97;
    let tilt = (r.unit() - 0.5) * 2.0e-3; // in [-1e-3, 1e-3]
    let c_conf = (conf + tilt).clamp(0.05, 0.999);
    let o_conf = (conf - tilt).clamp(0.05, 0.999);

    // Bloc 1 supports CORRECT, opposes OTHER.
    for i in 0..per {
        let id = format!("agent_{i}");
        wf = wf.vote(AgentVote::new(&id, CORRECT, c_conf, true));
        wf = wf.vote(AgentVote::new(&id, OTHER, o_conf, false));
    }
    // Bloc 2 supports OTHER, opposes CORRECT — mirror image, forcing a near-tie.
    for i in per..total {
        let id = format!("agent_{i}");
        wf = wf.vote(AgentVote::new(&id, OTHER, o_conf, true));
        wf = wf.vote(AgentVote::new(&id, CORRECT, c_conf, false));
    }
    wf
}

/// Run both methods over `SEEDS` instances of a regime and return their stats.
fn run_regime(build: impl Fn(u64) -> AgentWavefront) -> (Stats, Stats) {
    let mut interf = Stats::default();
    let mut selfc = Stats::default();
    for seed in 0..SEEDS {
        let wf = build(seed);
        interf.record(interference_pick(&wf, seed.wrapping_add(1)));
        selfc.record(self_consistency_pick(&wf));
    }
    (interf, selfc)
}

fn print_row(regime: &str, method: &str, s: &Stats) {
    println!(
        "| {regime:<16} | {method:<16} | {:>6.1}% | {:>6.1}% | {:>6.1}% |",
        s.accuracy() * 100.0,
        s.defer_rate() * 100.0,
        s.capture_rate() * 100.0,
    );
}

#[test]
fn interference_vs_self_consistency_headtohead() {
    let (h_i, h_s) = run_regime(regime_honest);
    let (c_i, c_s) = run_regime(regime_colluding);
    let (d_i, d_s) = run_regime(regime_deadlock);

    println!("\n=== interference-consensus vs self-consistency ===");
    println!("seeds/regime = {SEEDS}, equal agent budget per task\n");
    println!("| regime           | method           |   acc  | defer  | capture|");
    println!("|------------------|------------------|--------|--------|--------|");
    print_row("honest", "interference", &h_i);
    print_row("honest", "self-consistency", &h_s);
    print_row("colluding-bloc", "interference", &c_i);
    print_row("colluding-bloc", "self-consistency", &c_s);
    print_row("deadlock", "interference", &d_i);
    print_row("deadlock", "self-consistency", &d_s);
    println!();
    println!("acc = named the ground-truth CORRECT plan; defer = declined to act;");
    println!("capture = named the colluding bloc's BAD plan.\n");

    // --- (a) honest majority: assert PARITY. Both should be (near-)perfectly
    // correct. We do NOT claim an interference win here. -----------------------
    assert!(
        h_s.accuracy() >= 0.99,
        "self-consistency should ace the honest-majority regime, got {:.3}",
        h_s.accuracy()
    );
    assert!(
        h_i.accuracy() >= 0.99,
        "interference should also ace honest majority (parity), got {:.3}",
        h_i.accuracy()
    );
    // Parity, not superiority: neither beats the other on honest majority.
    assert!(
        (h_i.accuracy() - h_s.accuracy()).abs() <= 0.02,
        "expected parity on honest majority: interf={:.3} selfc={:.3}",
        h_i.accuracy(),
        h_s.accuracy()
    );

    // --- (b) colluding bloc: this is where interference + the gate should
    // genuinely help. Naive majority is captured by the larger BAD bloc;
    // interference must be captured strictly less often. ----------------------
    assert!(
        c_s.capture_rate() >= 0.99,
        "naive majority should be captured by the BAD bloc, got {:.3}",
        c_s.capture_rate()
    );
    assert!(
        c_i.capture_rate() < c_s.capture_rate(),
        "interference must resist capture better than majority: interf={:.3} selfc={:.3}",
        c_i.capture_rate(),
        c_s.capture_rate()
    );
    // The safe behavior under a colluding bloc is to NOT execute BAD: either
    // defer or pick CORRECT. Interference should essentially never be captured.
    assert!(
        c_i.capture_rate() <= 0.01,
        "interference should almost never execute the BAD bloc plan, got {:.3}",
        c_i.capture_rate()
    );

    // --- (c) deadlock / near-tie: interference should DEFER; it must not get
    // captured into a confident wrong answer. Majority picks arbitrarily. -----
    assert!(
        d_i.defer_rate() >= 0.9,
        "interference should defer on a near-tie, got defer={:.3}",
        d_i.defer_rate()
    );
    assert!(
        d_i.capture_rate() <= 0.01,
        "interference must not be captured on a deadlock, got {:.3}",
        d_i.capture_rate()
    );

    println!("VERDICT:");
    println!(
        "  honest-majority : PARITY  (interf {:.1}% vs selfc {:.1}% acc) — no win claimed",
        h_i.accuracy() * 100.0,
        h_s.accuracy() * 100.0
    );
    println!(
        "  colluding-bloc  : INTERFERENCE WINS (capture {:.1}% vs {:.1}%)",
        c_i.capture_rate() * 100.0,
        c_s.capture_rate() * 100.0
    );
    println!(
        "  deadlock        : INTERFERENCE DEFERS ({:.1}%); majority guesses (acc {:.1}%)",
        d_i.defer_rate() * 100.0,
        d_s.accuracy() * 100.0
    );
}
