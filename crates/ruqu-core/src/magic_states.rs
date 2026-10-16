//! Auditable magic-state factory baselines and resource accounting.
//!
//! The executable model is ideal 15-to-1 Reed-Muller distillation with independent
//! input Z faults, perfect Clifford operations and perfect syndrome measurement.
//! Exact integer dynamic programming accounts for all 2^15 fault patterns.
//! It is NOT a reproduction of Google's
//! physical magic-state cultivation experiment or an unfolded-distillation circuit.
//! See Bravyi and Kitaev, PRA 71, 022316 (2005), Sec. VI, equations 35-38:
//! <https://arxiv.org/abs/quant-ph/0403025>.

use std::{fmt, sync::OnceLock};

/// Invalid probabilities, impossible yield, or non-finite resource arithmetic.
#[derive(Debug, Clone, PartialEq)]
pub enum FactoryError {
    InvalidProbability,
    InvalidResources,
    EmptySamples,
    ZeroYield,
    Overflow,
    InvalidFaultMask,
    NumericalUnderflow,
}

impl fmt::Display for FactoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "magic-state factory error: {:?}", self)
    }
}
impl std::error::Error for FactoryError {}

fn probability(p: f64) -> Result<(), FactoryError> {
    if p.is_finite() && (0.0..=1.0).contains(&p) {
        Ok(())
    } else {
        Err(FactoryError::InvalidProbability)
    }
}
fn finite(x: f64) -> Result<f64, FactoryError> {
    if x.is_finite() {
        Ok(x)
    } else {
        Err(FactoryError::Overflow)
    }
}

/// Four syndrome bits and the logical parity of an input fault pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FaultOutcome {
    pub syndrome: u8,
    pub logical_error: bool,
}
impl FaultOutcome {
    pub fn accepted(self) -> bool {
        self.syndrome == 0
    }
}

/// Bit i marks a Z fault on input i. Columns of the check matrix are 1..=15.
pub fn classify_faults(mask: u16) -> Result<FaultOutcome, FactoryError> {
    if mask & 0x8000 != 0 {
        return Err(FactoryError::InvalidFaultMask);
    }
    let mut syndrome = 0;
    for i in 0..15 {
        if mask & (1 << i) != 0 {
            syndrome ^= (i + 1) as u8;
        }
    }
    Ok(FaultOutcome {
        syndrome,
        logical_error: mask.count_ones() % 2 == 1,
    })
}

/// Normalized probabilities under the documented independent input error model.
#[derive(Debug, Clone, Copy)]
pub struct DistillationResult {
    pub input_error: f64,
    pub acceptance_probability: f64,
    pub rejected_probability: f64,
    pub accepted_logical_error_probability: f64,
    pub conditional_output_error: f64,
}

/// Compute accepted harmless and harmful counts by Hamming weight with exact
/// integer dynamic programming over 16 syndromes, two parities and 16 weights.
/// Exposed separately so cold setup can be benchmarked without cache effects.
/// All counts are bounded by 2^15 and fit exactly in u32.
pub fn compute_weight_enumerators() -> ([u32; 16], [u32; 16]) {
    let mut counts = [[[0u32; 16]; 2]; 16];
    counts[0][0][0] = 1;
    for column in 1usize..=15 {
        // Descending weights ensure each column is applied at most once.
        for weight in (0..column).rev() {
            for syndrome in 0..16 {
                for parity in 0..2 {
                    let count = counts[syndrome][parity][weight];
                    counts[syndrome ^ column][parity ^ 1][weight + 1] += count;
                }
            }
        }
    }
    (counts[0][0], counts[0][1])
}

/// Cached exact accepted-pattern spectrum. Positive polynomial evaluation avoids
/// the cancellation of the equivalent closed form at tiny input error rates.
pub fn weight_enumerators() -> ([u32; 16], [u32; 16]) {
    static SPECTRUM: OnceLock<([u32; 16], [u32; 16])> = OnceLock::new();
    *SPECTRUM.get_or_init(compute_weight_enumerators)
}

/// Exact finite fault-pattern sum, up to floating point rounding.
pub fn distill_15_to_1(input_error: f64) -> Result<DistillationResult, FactoryError> {
    probability(input_error)?;
    let (good, bad) = weight_enumerators();
    let mut accepted = 0.0;
    let mut harmful = 0.0;
    for weight in 0..=15 {
        let mass = input_error.powi(weight as i32) * (1.0 - input_error).powi((15 - weight) as i32);
        accepted += (good[weight] + bad[weight]) as f64 * mass;
        harmful += bad[weight] as f64 * mass;
    }
    // A positive input error always has nonzero weight-three harmful mass.
    // Refuse a falsely perfect output if f64 cannot represent that mass.
    if input_error > 0.0 && harmful == 0.0 {
        return Err(FactoryError::NumericalUnderflow);
    }
    let accepted = accepted.clamp(0.0, 1.0);
    if accepted == 0.0 {
        return Err(FactoryError::ZeroYield);
    }
    Ok(DistillationResult {
        input_error,
        acceptance_probability: accepted,
        rejected_probability: 1.0 - accepted,
        accepted_logical_error_probability: harmful,
        conditional_output_error: (harmful / accepted).clamp(0.0, 1.0),
    })
}

/// Distribution-free two-sided Hoeffding interval. Zero observations give [0,1].
#[derive(Debug, Clone, Copy)]
pub struct ProbabilityInterval {
    pub lower: f64,
    pub upper: f64,
}

fn interval(successes: u64, trials: u64, log_failure_probability: f64) -> ProbabilityInterval {
    if trials == 0 {
        return ProbabilityInterval {
            lower: 0.0,
            upper: 1.0,
        };
    }
    let point = successes as f64 / trials as f64;
    let radius = ((2.0_f64.ln() - log_failure_probability) / (2.0 * trials as f64)).sqrt();
    ProbabilityInterval {
        lower: (point - radius).max(0.0),
        upper: (point + radius).min(1.0),
    }
}

/// Every attempted block is accounted for, including rejects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonteCarloCounts {
    pub attempts: u64,
    pub accepted: u64,
    pub rejected: u64,
    pub logical_errors: u64,
    pub seed: u64,
}
impl MonteCarloCounts {
    pub fn acceptance_estimate(&self) -> f64 {
        self.accepted as f64 / self.attempts as f64
    }
    pub fn output_error_estimate(&self) -> Option<f64> {
        if self.accepted == 0 {
            None
        } else {
            Some(self.logical_errors as f64 / self.accepted as f64)
        }
    }
    /// Each returned interval has failure probability <= alpha/2; the pair
    /// simultaneously covers with probability >= 1-alpha by the union bound.
    /// The output interval conditions on the observed accepted sample count.
    pub fn confidence_bounds(
        &self,
        alpha: f64,
    ) -> Result<(ProbabilityInterval, ProbabilityInterval), FactoryError> {
        if !alpha.is_finite() || alpha <= 0.0 || alpha >= 1.0 {
            return Err(FactoryError::InvalidProbability);
        }
        // Log-domain computation preserves tiny positive alpha values.
        let adjusted = alpha.ln() - 2.0_f64.ln();
        Ok((
            interval(self.accepted, self.attempts, adjusted),
            interval(self.logical_errors, self.accepted, adjusted),
        ))
    }
}

// SplitMix64 is explicitly specified here so seeded results do not depend on
// changing rand crate versions. This RNG is for simulation, never cryptography.
fn uniform(state: &mut u64) -> f64 {
    *state = state.wrapping_add(0x9e3779b97f4a7c15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    z ^= z >> 31;
    (z >> 11) as f64 * (1.0 / 9007199254740992.0)
}

pub fn sample_15_to_1(
    input_error: f64,
    attempts: u64,
    seed: u64,
) -> Result<MonteCarloCounts, FactoryError> {
    probability(input_error)?;
    if attempts == 0 {
        return Err(FactoryError::EmptySamples);
    }
    let mut result = MonteCarloCounts {
        attempts,
        accepted: 0,
        rejected: 0,
        logical_errors: 0,
        seed,
    };
    let mut state = seed;
    for _ in 0..attempts {
        let mut mask = 0u16;
        for i in 0..15 {
            if uniform(&mut state) < input_error {
                mask |= 1 << i;
            }
        }
        let outcome = classify_faults(mask)?;
        if outcome.accepted() {
            result.accepted += 1;
            if outcome.logical_error {
                result.logical_errors += 1;
            }
        } else {
            result.rejected += 1;
        }
    }
    Ok(result)
}

/// Costs per block attempt, including failed attempts. Units must be consistent
/// across stages. No hardware cost, routing or latency model is inferred.
#[derive(Debug, Clone, Copy)]
pub struct AttemptCost {
    pub qubits: f64,
    pub rounds: f64,
}
impl AttemptCost {
    fn volume(self) -> Result<f64, FactoryError> {
        if !self.qubits.is_finite()
            || !self.rounds.is_finite()
            || self.qubits < 0.0
            || self.rounds < 0.0
        {
            return Err(FactoryError::InvalidResources);
        }
        finite(self.qubits * self.rounds)
    }
}

/// A calibrated stage profile, supplied by the caller. Its output error bound
/// must already account for its input error model; composition does not invent
/// a suppression law. A literature number alone does not calibrate a new device.
#[derive(Debug, Clone, Copy)]
pub struct FactoryStage {
    pub input_states: u32,
    pub acceptance_probability: f64,
    pub conditional_output_error_upper_bound: f64,
    pub attempt_cost: AttemptCost,
}

#[derive(Debug, Clone, Copy)]
pub struct FactoryEstimate {
    pub expected_raw_states: f64,
    pub expected_stage_attempts: f64,
    pub expected_qubit_rounds: f64,
    pub output_error_upper_bound: f64,
}

/// Unlimited retry-until-success accounting. Each stage consumes input_states
/// independent previous-stage outputs on *every* attempt, including failures.
/// Values are expectations, not peak qubits or wall-clock latency guarantees.
pub fn estimate_factory(
    stages: &[FactoryStage],
    raw_state_cost: AttemptCost,
) -> Result<FactoryEstimate, FactoryError> {
    let mut result = FactoryEstimate {
        expected_raw_states: 1.0,
        expected_stage_attempts: 0.0,
        expected_qubit_rounds: raw_state_cost.volume()?,
        output_error_upper_bound: 1.0,
    };
    if stages.is_empty() {
        return Err(FactoryError::InvalidResources);
    }
    for stage in stages {
        probability(stage.acceptance_probability)?;
        probability(stage.conditional_output_error_upper_bound)?;
        if stage.acceptance_probability == 0.0 {
            return Err(FactoryError::ZeroYield);
        }
        if stage.input_states == 0 {
            return Err(FactoryError::InvalidResources);
        }
        let n = stage.input_states as f64;
        let a = stage.acceptance_probability;
        result.expected_raw_states = finite(n * result.expected_raw_states / a)?;
        result.expected_stage_attempts = finite((n * result.expected_stage_attempts + 1.0) / a)?;
        result.expected_qubit_rounds =
            finite((n * result.expected_qubit_rounds + stage.attempt_cost.volume()?) / a)?;
        result.output_error_upper_bound = stage.conditional_output_error_upper_bound;
    }
    Ok(result)
}

/// Account for a fallible transfer. Selection can concentrate upstream errors,
/// so use min(1, upstream_error / transfer_acceptance + transfer_error).
/// The transfer error input bounds newly introduced error conditional on success.
/// Failed transfer consumes the complete preparation and transfer attempt cost.
pub fn apply_transfer(
    mut estimate: FactoryEstimate,
    acceptance: f64,
    error_upper_bound: f64,
    cost: AttemptCost,
) -> Result<FactoryEstimate, FactoryError> {
    probability(acceptance)?;
    probability(error_upper_bound)?;
    probability(estimate.output_error_upper_bound)?;
    if acceptance == 0.0 {
        return Err(FactoryError::ZeroYield);
    }
    for x in [
        estimate.expected_raw_states,
        estimate.expected_stage_attempts,
        estimate.expected_qubit_rounds,
    ] {
        if !x.is_finite() || x < 0.0 {
            return Err(FactoryError::InvalidResources);
        }
    }
    estimate.expected_raw_states = finite(estimate.expected_raw_states / acceptance)?;
    estimate.expected_stage_attempts =
        finite((estimate.expected_stage_attempts + 1.0) / acceptance)?;
    estimate.expected_qubit_rounds =
        finite((estimate.expected_qubit_rounds + cost.volume()?) / acceptance)?;
    estimate.output_error_upper_bound =
        (estimate.output_error_upper_bound / acceptance + error_upper_bound).min(1.0);
    Ok(estimate)
}

/// Compose ideal independent-input 15-to-1 stages and calculate their errors.
pub fn distillation_factory(
    input_error: f64,
    rounds: usize,
    block_cost: AttemptCost,
    raw_state_cost: AttemptCost,
) -> Result<FactoryEstimate, FactoryError> {
    probability(input_error)?;
    if rounds == 0 {
        return Err(FactoryError::InvalidResources);
    }
    // Streaming composition avoids allocating a user-controlled stage vector.
    let mut result = FactoryEstimate {
        expected_raw_states: 1.0,
        expected_stage_attempts: 0.0,
        expected_qubit_rounds: raw_state_cost.volume()?,
        output_error_upper_bound: input_error,
    };
    for _ in 0..rounds {
        let distilled = distill_15_to_1(result.output_error_upper_bound)?;
        let one = estimate_factory(
            &[FactoryStage {
                input_states: 15,
                acceptance_probability: distilled.acceptance_probability,
                conditional_output_error_upper_bound: distilled.conditional_output_error,
                attempt_cost: block_cost,
            }],
            AttemptCost {
                qubits: 0.0,
                rounds: 0.0,
            },
        )?;
        result.expected_raw_states = finite(one.expected_raw_states * result.expected_raw_states)?;
        result.expected_stage_attempts = finite(
            one.expected_raw_states * result.expected_stage_attempts + one.expected_stage_attempts,
        )?;
        result.expected_qubit_rounds = finite(
            one.expected_raw_states * result.expected_qubit_rounds + one.expected_qubit_rounds,
        )?;
        result.output_error_upper_bound = distilled.conditional_output_error;
    }
    Ok(result)
}
