//! Exact dense trajectories for the complete f=3 cultivation stage in
//! arXiv:2609.16929v1, Table 3 and Figures 7, 18, 19(a).
//! No escape stage or fictitious error correction is included. Output metrics
//! are full encoded-state overlap and raw codespace/logical probabilities.
use crate::{gate::Gate, state::QuantumState, types::Complex};
use rand::{rngs::StdRng, Rng, SeedableRng};

pub const SURFACE_X_CHECKS: [u16; 6] = [
    (1 << 0) | (1 << 1) | (1 << 9),
    (1 << 1) | (1 << 2) | (1 << 10),
    (1 << 3) | (1 << 9) | (1 << 4) | (1 << 11),
    (1 << 4) | (1 << 10) | (1 << 5) | (1 << 12),
    (1 << 6) | (1 << 11) | (1 << 7),
    (1 << 7) | (1 << 12) | (1 << 8),
];
pub const SURFACE_Z_CHECKS: [u16; 6] = [
    (1 << 0) | (1 << 3) | (1 << 9),
    (1 << 1) | (1 << 9) | (1 << 4) | (1 << 10),
    (1 << 2) | (1 << 10) | (1 << 5),
    (1 << 3) | (1 << 11) | (1 << 6),
    (1 << 4) | (1 << 11) | (1 << 7) | (1 << 12),
    (1 << 5) | (1 << 12) | (1 << 8),
];
pub const PROTOCOL_ID: &str = "fold-transversal-surface-f3-2609.16929v1";

#[derive(Clone, Debug, PartialEq)]
pub struct CultivationConfig {
    pub attempts: u32,
    pub seed: u64,
    /// Total depolarizing probability after each physical one/two-qubit gate.
    pub gate_error: f64,
    /// X error after each physical |0> preparation.
    pub preparation_error: f64,
    /// X error immediately before each Z readout.
    pub measurement_error: f64,
    pub verification_rounds: u32,
}
impl Default for CultivationConfig {
    fn default() -> Self {
        Self {
            attempts: 100,
            seed: 42,
            gate_error: 0.0,
            preparation_error: 0.0,
            measurement_error: 0.0,
            verification_rounds: 2,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct CultivationReport {
    pub protocol_id: &'static str,
    pub attempts: u32,
    pub accepted: u32,
    pub rejected: u32,
    /// Rejection at initial syndrome / logical H / final syndrome, respectively.
    pub rejected_by_stage: [u32; 3],
    pub acceptance_probability: f64,
    pub acceptance_wilson_95: [f64; 2],
    /// Full encoded pure-state fidelity, including leakage out of the codespace.
    pub mean_accepted_fidelity: Option<f64>,
    pub mean_codespace_probability: Option<f64>,
    /// Joint probability of codespace membership and logical H=-1.
    pub mean_logical_h_minus_probability: Option<f64>,
    pub physical_gates: u64,
    pub measurements: u64,
    pub preparations: u64,
    pub sampled_fault_locations: u64,
    pub executed_qubit_operations: u64,
    pub peak_state_bytes: usize,
    /// Bernoulli oracle samples of full output infidelity among accepted shots.
    pub sampled_output_error_events: u32,
    pub output_error_wilson_95: Option<[f64; 2]>,
}
struct Executor {
    state: QuantumState,
    rng: StdRng,
    gate_error: f64,
    prep_error: f64,
    measure_error: f64,
    gates: u64,
    measures: u64,
    preps: u64,
    faults: u64,
    work: u64,
    postselect: bool,
    branch_probability: f64,
    fault_at: Option<(u64, u32)>,
    locations: Vec<(u64, u32)>,
    trace: Vec<(String, Vec<u32>)>,
}
impl Executor {
    fn pauli(&mut self, q: u32, p: u32) {
        let g = match p {
            1 => Gate::X(q),
            2 => Gate::Y(q),
            3 => Gate::Z(q),
            _ => return,
        };
        self.state.apply_gate(&g).expect("fixed in-range Pauli");
    }
    fn location(&mut self, qs: &[u32], max: u32) {
        let location = self.locations.len() as u64;
        self.locations.push((location, max));
        if let Some((at, mut p)) = self.fault_at {
            if location == at {
                for &q in qs {
                    self.pauli(q, p & 3);
                    p >>= 2;
                }
            }
        }
    }
    fn gate(&mut self, gate: Gate) {
        let qs = gate.qubits();
        self.trace.push((format!("{gate:?}"), qs.clone()));
        self.state
            .apply_gate(&gate)
            .expect("fixed in-range circuit gate");
        self.gates += 1;
        self.work += qs.len() as u64;
        self.location(&qs, (1u32 << (2 * qs.len())) - 1);
        if self.rng.gen::<f64>() < self.gate_error {
            self.faults += 1;
            // Uniformly choose all 4^k-1 nonidentity Pauli products.
            let mut p = self.rng.gen_range(1..(1u32 << (2 * qs.len())));
            for q in qs {
                self.pauli(q, p & 3);
                p >>= 2;
            }
        }
    }
    fn prep(&mut self, q: u32) {
        self.trace.push(("Prepare".into(), vec![q]));
        self.state.reset_qubit(q).expect("fixed in-range reset");
        self.preps += 1;
        self.work += 1;
        self.location(&[q], 1);
        if self.rng.gen::<f64>() < self.prep_error {
            self.pauli(q, 1);
            self.faults += 1;
        }
    }
    fn measure(&mut self, q: u32) -> bool {
        self.trace.push(("Measure".into(), vec![q]));
        self.location(&[q], 1);
        if self.rng.gen::<f64>() < self.measure_error {
            self.pauli(q, 1);
            self.faults += 1;
        }
        self.measures += 1;
        self.work += 1;
        if self.postselect {
            let p0 = self
                .state
                .state_vector()
                .iter()
                .enumerate()
                .filter(|(i, _)| i & (1 << q) == 0)
                .map(|(_, a)| a.norm_sq())
                .sum::<f64>()
                .clamp(0.0, 1.0);
            self.branch_probability *= p0;
            if p0 < 1e-25 {
                self.branch_probability = 0.0;
                return true;
            }
            for (i, a) in self.state.amplitudes_mut().iter_mut().enumerate() {
                *a = if i & (1 << q) == 0 {
                    *a * (1.0 / p0.sqrt())
                } else {
                    Complex::ZERO
                };
            }
            false
        } else {
            self.state
                .measure(q)
                .expect("fixed in-range measurement")
                .result
        }
    }
    fn check(&mut self, mask: u16, x: bool) -> bool {
        self.prep(13);
        if x {
            self.gate(Gate::H(13));
        }
        for q in 0..13 {
            if mask & (1 << q) != 0 {
                self.gate(if x {
                    Gate::CNOT(13, q)
                } else {
                    Gate::CNOT(q, 13)
                });
            }
        }
        if x {
            self.gate(Gate::H(13));
        }
        !self.measure(13)
    }
    fn syndrome(&mut self) -> bool {
        for mask in SURFACE_X_CHECKS {
            if !self.check(mask, true) {
                return false;
            }
        }
        for mask in SURFACE_Z_CHECKS {
            if !self.check(mask, false) {
                return false;
            }
        }
        true
    }
    fn ccz(&mut self, a: u32, b: u32, c: u32) {
        self.trace.push(("CCZ".into(), vec![a, b, c]));
        for (i, amp) in self.state.amplitudes_mut().iter_mut().enumerate() {
            if i & (1 << a) != 0 && i & (1 << b) != 0 && i & (1 << c) != 0 {
                *amp = -*amp;
            }
        }
        self.gates += 1;
        self.work += 3;
        self.location(&[a, b, c], 63);
        if self.rng.gen::<f64>() < self.gate_error {
            self.faults += 1;
            let mut p = self.rng.gen_range(1..64);
            for q in [a, b, c] {
                self.pauli(q, p & 3);
                p >>= 2;
            }
        }
    }
    fn h_check(&mut self) -> bool {
        // Figure 19(a), physical cat ordering a0=13,a1=14,a2=15.
        for q in 13..16 {
            self.prep(q);
        }
        self.gate(Gate::H(14));
        self.gate(Gate::CNOT(14, 15));
        self.gate(Gate::CNOT(14, 13));
        // Eq.191 then Eq.192, Table4 offsets s=(0,0,2).
        for (j, q) in [0, 9, 4, 12, 8].into_iter().enumerate() {
            let a = 13 + (j % 3) as u32;
            self.gate(if j % 2 == 0 { Gate::Tdg(q) } else { Gate::T(q) });
            self.gate(Gate::CNOT(a, q));
            self.gate(if j % 2 == 0 { Gate::T(q) } else { Gate::Tdg(q) });
        }
        for (j, (a, b)) in [(3, 1), (11, 10), (7, 5)].into_iter().enumerate() {
            self.ccz(13 + j as u32, a, b);
        }
        self.ccz(15, 6, 2);
        self.gate(Gate::CNOT(14, 13));
        self.gate(Gate::CNOT(14, 15));
        self.gate(Gate::H(14));
        // All three bits trivial, including cat verification outcomes.
        let mut ok = true;
        for q in 13..16 {
            ok &= !self.measure(q);
        }
        ok
    }
    fn inject_and_grow(&mut self) {
        // Figure18, time ordered CNOT columns. Simultaneous disjoint gates
        // commute; they are listed lexicographically inside each column.
        for q in [0, 2, 4, 5, 6] {
            self.gate(Gate::H(q));
        }
        self.gate(Gate::T(4));
        for (a, b) in [
            (0, 3),
            (5, 8),
            (6, 7),
            (4, 1),
            (5, 4),
            (1, 7),
            (2, 1),
            (0, 4),
            (5, 7),
            (0, 1),
        ] {
            self.gate(Gate::CNOT(a, b));
        }
        // Figure7 half-cycle. Added data are square centers 9,10,11,12.
        self.gate(Gate::H(9));
        self.gate(Gate::H(12));
        for (a, b) in [
            (9, 0),
            (1, 10),
            (3, 11),
            (12, 4),
            (9, 1),
            (4, 10),
            (6, 11),
            (12, 5),
        ] {
            self.gate(Gate::CNOT(a, b));
        }
    }
}
/// Independent CSS row-space construction, not derived by the injection circuit.
/// Logical X is the left boundary (0,3,6), logical Z the top boundary (0,1,2).
pub fn surface_reference_codewords() -> (Vec<Complex>, Vec<Complex>) {
    let mut zero = vec![Complex::ZERO; 8192];
    let mut one = zero.clone();
    for word in 0..64 {
        let mut index = 0u16;
        for (j, row) in SURFACE_X_CHECKS.iter().enumerate() {
            if word & (1 << j) != 0 {
                index ^= *row;
            }
        }
        zero[index as usize] = Complex::new(0.125, 0.0);
        one[(index ^ ((1 << 0) | (1 << 3) | (1 << 6))) as usize] = Complex::new(0.125, 0.0);
    }
    (zero, one)
}
/// [full T-state fidelity, codespace probability, joint codespace/HXY=-1].
pub fn surface_metrics(state: &[Complex]) -> Result<[f64; 3], String> {
    if state.len() != 8192 && state.len() != 65536 {
        return Err("expected 13 or 16 qubits".into());
    }
    let (zero, one) = surface_reference_codewords();
    let phase = Complex::from_polar(1.0, -std::f64::consts::FRAC_PI_4);
    let (mut fidelity, mut codespace) = (0.0, 0.0);
    for block in state.chunks_exact(8192) {
        let mut a = Complex::ZERO;
        let mut b = Complex::ZERO;
        for i in 0..8192 {
            a += zero[i].conj() * block[i];
            b += one[i].conj() * block[i];
        }
        fidelity += ((a + phase * b) * std::f64::consts::FRAC_1_SQRT_2).norm_sq();
        codespace += a.norm_sq() + b.norm_sq();
    }
    Ok([
        fidelity.clamp(0.0, 1.0),
        codespace.clamp(0.0, 1.0),
        (codespace - fidelity).clamp(0.0, 1.0),
    ])
}
/// A noiseless state after injection and code growth, for independent references.
pub fn surface_injected_state() -> Vec<Complex> {
    let mut ex = Executor {
        state: QuantumState::new_with_seed(16, 0).expect("16 qubits"),
        rng: StdRng::seed_from_u64(0),
        gate_error: 0.0,
        prep_error: 0.0,
        measure_error: 0.0,
        gates: 0,
        measures: 0,
        preps: 0,
        faults: 0,
        work: 0,
        postselect: false,
        branch_probability: 1.0,
        fault_at: None,
        locations: vec![],
        trace: vec![],
    };
    ex.inject_and_grow();
    ex.state.state_vector()[..8192].to_vec()
}
fn wilson(events: u32, trials: u32) -> [f64; 2] {
    let n = trials as f64;
    let p = events as f64 / n;
    let z2 = 3.841458820694124;
    let den = 1.0 + z2 / n;
    let center = (p + z2 / (2.0 * n)) / den;
    let radius = (z2 * (p * (1.0 - p) / n + z2 / (4.0 * n * n))).sqrt() / den;
    [(center - radius).max(0.0), (center + radius).min(1.0)]
}
/// Monte Carlo exact state-vector trajectories with explicit early rejection.
/// Bounds: at most 10000 attempts, 16 qubits, exactly two logical-H checks per attempt.
/// Rejected attempts are counted and their already-executed work remains charged.
pub fn run_surface_cultivation(config: &CultivationConfig) -> Result<CultivationReport, String> {
    if config.attempts == 0 || config.attempts > 10000 {
        return Err("attempts must be in 1..=10000".into());
    }
    if config.verification_rounds != 2 {
        return Err("published f3 protocol requires verification_rounds=2".into());
    }
    for p in [
        config.gate_error,
        config.preparation_error,
        config.measurement_error,
    ] {
        if !p.is_finite() || !(0.0..=1.0).contains(&p) {
            return Err("noise probabilities must be finite and in [0,1]".into());
        }
    }
    let mut report = CultivationReport {
        protocol_id: PROTOCOL_ID,
        attempts: config.attempts,
        accepted: 0,
        rejected: 0,
        rejected_by_stage: [0; 3],
        acceptance_probability: 0.0,
        acceptance_wilson_95: [0.0; 2],
        mean_accepted_fidelity: None,
        mean_codespace_probability: None,
        mean_logical_h_minus_probability: None,
        physical_gates: 0,
        measurements: 0,
        preparations: 0,
        sampled_fault_locations: 0,
        executed_qubit_operations: 0,
        peak_state_bytes: 65536 * std::mem::size_of::<Complex>(),
        sampled_output_error_events: 0,
        output_error_wilson_95: None,
    };
    let mut root = StdRng::seed_from_u64(config.seed);
    let mut sums = [0.0; 3];
    for _ in 0..config.attempts {
        let mut ex = Executor {
            state: QuantumState::new_with_seed(16, root.gen()).map_err(|e| e.to_string())?,
            rng: StdRng::seed_from_u64(root.gen()),
            gate_error: config.gate_error,
            prep_error: config.preparation_error,
            measure_error: config.measurement_error,
            gates: 0,
            measures: 0,
            preps: 0,
            faults: 0,
            work: 0,
            postselect: false,
            branch_probability: 1.0,
            fault_at: None,
            locations: vec![],
            trace: vec![],
        };
        for q in 0..13 {
            ex.prep(q);
        }
        ex.inject_and_grow();
        let stage = if !ex.syndrome() {
            Some(0)
        } else {
            let mut ok = true;
            for _ in 0..config.verification_rounds {
                if !ex.h_check() {
                    ok = false;
                    break;
                }
            }
            if !ok {
                Some(1)
            } else if !ex.syndrome() {
                Some(2)
            } else {
                None
            }
        };
        if let Some(stage) = stage {
            report.rejected += 1;
            report.rejected_by_stage[stage] += 1;
        } else {
            report.accepted += 1;
            let m = surface_metrics(ex.state.state_vector())?;
            for i in 0..3 {
                sums[i] += m[i];
            }
            if root.gen::<f64>() < 1.0 - m[0] {
                report.sampled_output_error_events += 1;
            }
        }
        report.physical_gates += ex.gates;
        report.measurements += ex.measures;
        report.preparations += ex.preps;
        report.sampled_fault_locations += ex.faults;
        report.executed_qubit_operations += ex.work;
    }
    report.acceptance_probability = report.accepted as f64 / config.attempts as f64;
    report.acceptance_wilson_95 = wilson(report.accepted, report.attempts);
    if report.accepted > 0 {
        let a = report.accepted as f64;
        report.mean_accepted_fidelity = Some(sums[0] / a);
        report.mean_codespace_probability = Some(sums[1] / a);
        report.mean_logical_h_minus_probability = Some(sums[2] / a);
        report.output_error_wilson_95 =
            Some(wilson(report.sampled_output_error_events, report.accepted));
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cat_gadget_rejects_the_orthogonal_logical_magic_state() {
        let mut ex = Executor {
            state: QuantumState::new_with_seed(16, 13).unwrap(),
            rng: StdRng::seed_from_u64(0),
            gate_error: 0.0,
            prep_error: 0.0,
            measure_error: 0.0,
            gates: 0,
            measures: 0,
            preps: 0,
            faults: 0,
            work: 0,
            postselect: false,
            branch_probability: 1.0,
            fault_at: None,
            locations: vec![],
            trace: vec![],
        };
        ex.inject_and_grow();
        // Logical Z along top boundary changes T+ to its orthogonal T-.
        for q in [0, 1, 2] {
            ex.pauli(q, 3);
        }
        assert!(!ex.h_check(), "cat gadget must distinguish +/- logical HXY");
    }
}

/// Deterministic all-zero accepted branch with one specified physical fault.
/// The code is base-four little-endian I/X/Y/Z over a gate's qubits; preparation
/// and readout locations support the X fault only. None gives the ideal circuit.
#[derive(Clone, Debug)]
pub struct SingleFaultAudit {
    pub circuit_locations: u64,
    pub fault_mechanisms: u64,
    pub accepted_fault_mechanisms: u64,
    pub nonzero_accepted_raw_infidelity: u64,
    pub nonzero_accepted_logical_error: u64,
    pub max_acceptance_weighted_raw_infidelity: f64,
    pub max_acceptance_weighted_logical_error: f64,
    pub first_logical_error: Option<(u64, u32)>,
}
fn exact_fault_branch(
    fault: Option<(u64, u32)>,
) -> (f64, [f64; 3], Vec<(u64, u32)>, Vec<(String, Vec<u32>)>) {
    let mut ex = Executor {
        state: QuantumState::new_with_seed(16, 0).expect("16 qubits"),
        rng: StdRng::seed_from_u64(0),
        gate_error: 0.0,
        prep_error: 0.0,
        measure_error: 0.0,
        gates: 0,
        measures: 0,
        preps: 0,
        faults: 0,
        work: 0,
        postselect: true,
        branch_probability: 1.0,
        fault_at: fault,
        locations: vec![],
        trace: vec![],
    };
    for q in 0..13 {
        ex.prep(q);
    }
    ex.inject_and_grow();
    let accepted = ex.syndrome() && ex.h_check() && ex.h_check() && ex.syndrome();
    let metrics = if accepted {
        surface_metrics(ex.state.state_vector()).expect("16-qubit state")
    } else {
        [0.0; 3]
    };
    (
        if accepted { ex.branch_probability } else { 0.0 },
        metrics,
        ex.locations,
        ex.trace,
    )
}
/// Exhaustive exact accepted-branch check over every single physical Pauli fault.
/// Joint codespace/HXY-minus corresponds to an ideal final error-detection oracle,
/// not to recovery of residual correctable errors. This is not a distance-3 proof:
/// it checks first-order mechanisms only and not every pair of circuit faults.
pub fn audit_surface_single_faults() -> SingleFaultAudit {
    let (_, _, locations, _) = exact_fault_branch(None);
    let mut report = SingleFaultAudit {
        circuit_locations: locations.len() as u64,
        fault_mechanisms: 0,
        accepted_fault_mechanisms: 0,
        nonzero_accepted_raw_infidelity: 0,
        nonzero_accepted_logical_error: 0,
        max_acceptance_weighted_raw_infidelity: 0.0,
        max_acceptance_weighted_logical_error: 0.0,
        first_logical_error: None,
    };
    for (location, max) in locations {
        for code in 1..=max {
            let (accept, m, _, _) = exact_fault_branch(Some((location, code)));
            report.fault_mechanisms += 1;
            if accept > 1e-12 {
                report.accepted_fault_mechanisms += 1;
            }
            let raw = accept * (1.0 - m[0]);
            let logical = accept * m[2];
            if raw > 1e-10 {
                report.nonzero_accepted_raw_infidelity += 1;
            }
            if logical > 1e-10 {
                report.nonzero_accepted_logical_error += 1;
                if report.first_logical_error.is_none() {
                    report.first_logical_error = Some((location, code));
                }
            }
            report.max_acceptance_weighted_raw_infidelity =
                report.max_acceptance_weighted_raw_infidelity.max(raw);
            report.max_acceptance_weighted_logical_error =
                report.max_acceptance_weighted_logical_error.max(logical);
        }
    }
    report
}

/// Complete physical circuit trace. One entry corresponds to one fault location.
/// Qubit order is the base-four Pauli-code order used by surface_fault_branch.
/// Gate names include Debug arguments; consumers should parse the leading name.
pub fn surface_circuit_trace() -> Vec<(String, Vec<u32>)> {
    exact_fault_branch(None).3
}
/// Exact all-zero branch probability and output metrics for one physical fault.
pub fn surface_fault_branch(location: u64, code: u32) -> Result<(f64, [f64; 3]), String> {
    let (_, _, locations, _) = exact_fault_branch(None);
    let &(_, max) = locations
        .get(location as usize)
        .ok_or("fault location out of range")?;
    if code == 0 || code > max {
        return Err("fault code out of range".into());
    }
    let (p, m, _, _) = exact_fault_branch(Some((location, code)));
    Ok((p, m))
}

/// Exact ideal all-zero branch probability and output metrics.
pub fn surface_ideal_branch() -> (f64, [f64; 3]) {
    let (p, m, _, _) = exact_fault_branch(None);
    (p, m)
}
