//! Exact, budgeted Heisenberg propagation of sparse Pauli observables.
//!
//! Clifford gates preserve one Pauli term. T gates and postselected Pauli
//! measurements may branch exponentially; a budget error never returns an
//! approximate answer. Noise is stochastic Pauli noise, including correlated
//! strings. This is not a reproduction of Willow cultivation or the general
//! Clifford-stabilizer algorithm of arXiv:2609.16929.
use crate::{
    error::{QuantumError, Result},
    gate::Gate,
};
use std::collections::BTreeMap;

/// I=0, X=1, Z=2, Y=3, with qubit zero first.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Pauli(pub Vec<u8>);
impl Pauli {
    pub fn identity(n: usize) -> Self {
        Self(vec![0; n])
    }
    pub fn parse(s: &str) -> Result<Self> {
        let p = s
            .chars()
            .map(|c| match c {
                'I' => Ok(0),
                'X' => Ok(1),
                'Z' => Ok(2),
                'Y' => Ok(3),
                _ => Err(invalid("expected I/X/Y/Z")),
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self(p))
    }
    fn commutes(&self, other: &Self) -> bool {
        self.0
            .iter()
            .zip(&other.0)
            .filter(|(a, b)| **a != 0 && **b != 0 && a != b)
            .count()
            % 2
            == 0
    }
    /// Product of commuting Hermitian strings, including its real sign.
    fn product(&self, other: &Self) -> (Self, f64) {
        let mut phase = 0u8;
        let bits = self
            .0
            .iter()
            .zip(&other.0)
            .map(|(&a, &b)| {
                phase = (phase
                    + match (a, b) {
                        (1, 3) | (3, 2) | (2, 1) => 1,
                        (3, 1) | (2, 3) | (1, 2) => 3,
                        _ => 0,
                    })
                    % 4;
                a ^ b
            })
            .collect();
        debug_assert!(phase == 0 || phase == 2);
        (Self(bits), if phase == 2 { -1.0 } else { 1.0 })
    }
}
#[derive(Clone, Copy, Debug)]
pub enum InputState {
    Zero,
    Plus,
    T,
}
#[derive(Clone, Debug)]
pub enum Operation {
    Gate(Gate),
    /// Apply P with probability p, otherwise identity. Mechanisms independent.
    PauliFault {
        pauli: Pauli,
        probability: f64,
    },
    /// Keep outcome +1 (positive=true) or -1. Counts towards acceptance.
    Postselect {
        pauli: Pauli,
        positive: bool,
    },
}
#[derive(Clone, Copy, Debug)]
pub struct Budget {
    pub max_terms: usize,
    pub max_work: usize,
}
impl Default for Budget {
    fn default() -> Self {
        Self {
            max_terms: 8192,
            max_work: 32_000_000,
        }
    }
}
#[derive(Debug)]
pub struct PropagationResult {
    pub acceptance_probability: f64,
    pub weighted_expectation: f64,
    /// None when selected branch has numerically zero probability.
    pub conditional_expectation: Option<f64>,
    pub peak_terms: usize,
    pub work: usize,
}
fn invalid(s: &str) -> QuantumError {
    QuantumError::CircuitError(s.into())
}
fn valid_probability(p: f64) -> bool {
    p.is_finite() && (0.0..=1.0).contains(&p)
}
fn add(terms: &mut BTreeMap<Pauli, f64>, p: Pauli, c: f64) {
    if c != 0.0 {
        *terms.entry(p).or_default() += c;
    }
}
fn conjugate(mut p: Pauli, g: &Gate) -> Result<Vec<(Pauli, f64)>> {
    let mut sign = 1.0;
    match *g {
        Gate::H(q) => {
            let v = &mut p.0[q as usize];
            if *v == 3 {
                sign = -1.0;
            }
            *v = ((*v & 1) << 1) | ((*v >> 1) & 1);
        }
        Gate::S(q) | Gate::Sdg(q) => {
            let v = &mut p.0[q as usize];
            if (*v == 1 && matches!(g, Gate::S(_))) || (*v == 3 && matches!(g, Gate::Sdg(_))) {
                sign = -1.0;
            }
            if *v & 1 != 0 {
                *v ^= 2;
            }
        }
        Gate::X(q) | Gate::Y(q) | Gate::Z(q) => {
            let a = p.0[q as usize];
            let b = match g {
                Gate::X(_) => 1,
                Gate::Y(_) => 3,
                _ => 2,
            };
            if a != 0 && a != b {
                sign = -1.0;
            }
        }
        Gate::CNOT(c, t) => {
            let a = p.0[c as usize];
            let b = p.0[t as usize];
            let xc = a & 1;
            let zc = a >> 1;
            let xt = b & 1;
            let zt = b >> 1;
            if xc & zt & (xt ^ zc ^ 1) != 0 {
                sign = -1.0;
            }
            p.0[t as usize] ^= xc;
            p.0[c as usize] ^= zt << 1;
        }
        Gate::CZ(c, t) => {
            // U=H(t) CNOT(c,t) H(t); this gate is self adjoint.
            let mut a = conjugate(p, &Gate::H(t))?.remove(0);
            let b = conjugate(a.0, &Gate::CNOT(c, t))?.remove(0);
            a = (b.0, a.1 * b.1);
            let b = conjugate(a.0, &Gate::H(t))?.remove(0);
            return Ok(vec![(b.0, a.1 * b.1)]);
        }
        Gate::SWAP(a, b) => p.0.swap(a as usize, b as usize),
        Gate::T(q) | Gate::Tdg(q) => {
            let q = q as usize;
            let v = p.0[q];
            if v & 1 != 0 {
                let mut other = p.clone();
                other.0[q] ^= 2;
                let other_sign = if (v == 1 && matches!(g, Gate::T(_)))
                    || (v == 3 && matches!(g, Gate::Tdg(_)))
                {
                    -1.0
                } else {
                    1.0
                };
                return Ok(vec![
                    (p, std::f64::consts::FRAC_1_SQRT_2),
                    (other, other_sign * std::f64::consts::FRAC_1_SQRT_2),
                ]);
            }
        }
        Gate::Barrier => {}
        _ => {
            return Err(invalid(
                "unsupported gate: use Clifford/T gates and explicit Pauli postselection",
            ))
        }
    }
    Ok(vec![(p, sign)])
}
fn scale(a: f64, b: f64) -> Result<f64> {
    let result = a * b;
    if !result.is_finite() || (a != 0.0 && b != 0.0 && result.abs() < f64::MIN_POSITIVE) {
        return Err(invalid("Pauli coefficient numerical underflow or overflow"));
    }
    Ok(result)
}
fn initial_expectation(p: &Pauli, inputs: &[InputState]) -> Result<f64> {
    let mut magic_factors = 0;
    for (&v, s) in p.0.iter().zip(inputs) {
        match (v, s) {
            (0, _) | (2, InputState::Zero) | (1, InputState::Plus) => {}
            (1, InputState::T) | (3, InputState::T) => magic_factors += 1,
            _ => return Ok(0.0),
        }
    }
    let mut result = 1.0;
    for _ in 0..magic_factors {
        result = scale(result, std::f64::consts::FRAC_1_SQRT_2)?;
    }
    Ok(result)
}
fn propagate(
    inputs: &[InputState],
    ops: &[Operation],
    observable: Pauli,
    budget: Budget,
) -> Result<(f64, usize, usize)> {
    let mut terms = BTreeMap::from([(observable, 1.0)]);
    let mut peak = 1;
    let mut work = 0usize;
    for op in ops.iter().rev() {
        work = work
            .checked_add(
                terms
                    .len()
                    .checked_mul(inputs.len().max(1))
                    .ok_or_else(|| invalid("work overflow"))?,
            )
            .ok_or_else(|| invalid("work overflow"))?;
        if work > budget.max_work {
            return Err(invalid("Pauli propagation work budget exhausted"));
        }
        let mut next = BTreeMap::new();
        for (p, c) in terms {
            match op {
                Operation::Gate(g) => {
                    for (p, s) in conjugate(p, g)? {
                        add(&mut next, p, scale(c, s)?);
                    }
                }
                Operation::PauliFault { pauli, probability } => {
                    let factor = if p.commutes(pauli) {
                        1.0
                    } else {
                        1.0 - 2.0 * probability
                    };
                    add(&mut next, p, scale(c, factor)?);
                }
                Operation::Postselect { pauli, positive } => {
                    if p.commutes(pauli) {
                        let (prod, sign) = pauli.product(&p);
                        add(&mut next, p, scale(c, 0.5)?);
                        add(
                            &mut next,
                            prod,
                            scale(c, 0.5)? * sign * if *positive { 1.0 } else { -1.0 },
                        );
                    }
                }
            }
            if next.len() > budget.max_terms {
                return Err(invalid("Pauli propagation term budget exhausted"));
            }
        }
        // Remove exact cancellations only; no floating point pruning.
        next.retain(|_, c| *c != 0.0);
        peak = peak.max(next.len());
        terms = next;
    }
    work = work
        .checked_add(
            terms
                .len()
                .checked_mul(inputs.len())
                .ok_or_else(|| invalid("work overflow"))?,
        )
        .ok_or_else(|| invalid("work overflow"))?;
    if work > budget.max_work {
        return Err(invalid("Pauli reduction work budget exhausted"));
    }
    let mut expected = 0.0;
    for (p, c) in &terms {
        expected += scale(*c, initial_expectation(p, inputs)?)?;
    }
    Ok((expected, peak, work))
}
/// Evaluate Tr(O rho_selected)/Tr(rho_selected), exactly up to f64 rounding.
/// General T circuits and repeated postselection may have exponential term growth.
pub fn evaluate(
    inputs: &[InputState],
    ops: &[Operation],
    observable: &Pauli,
    budget: Budget,
) -> Result<PropagationResult> {
    let n = inputs.len();
    if n == 0
        || n > 4096
        || budget.max_terms == 0
        || budget.max_terms > 65536
        || budget.max_work == 0
    {
        return Err(invalid("invalid qubit count or budget"));
    }
    if ops.len() > 1_000_000 {
        return Err(invalid("too many operations"));
    }
    if n.saturating_mul(budget.max_terms) > 16_777_216 {
        return Err(invalid("Pauli storage budget exceeded"));
    }
    let validation_work = n
        .checked_mul(ops.len() + 3)
        .ok_or_else(|| invalid("validation work overflow"))?;
    if validation_work >= budget.max_work {
        return Err(invalid("Pauli propagation validation budget exhausted"));
    }
    let budget = Budget {
        max_work: budget.max_work - validation_work,
        ..budget
    };
    let check = |p: &Pauli| {
        if p.0.len() == n && p.0.iter().all(|v| *v <= 3) {
            Ok(())
        } else {
            Err(invalid("invalid Pauli width or element"))
        }
    };
    check(observable)?;
    for op in ops {
        match op {
            Operation::Gate(g) => {
                let qs = g.qubits();
                if qs.iter().any(|q| *q as usize >= n) {
                    return Err(invalid("gate qubit out of range"));
                }
                if qs.len() == 2 && qs[0] == qs[1] {
                    return Err(invalid("two qubit gate requires distinct qubits"));
                }
                conjugate(Pauli::identity(n), g)?;
            }
            Operation::PauliFault { pauli, probability } => {
                check(pauli)?;
                if !valid_probability(*probability) {
                    return Err(invalid("invalid Pauli fault probability"));
                }
            }
            Operation::Postselect { pauli, .. } => check(pauli)?,
        }
    }
    let (accept, peak_a, work_a) = propagate(inputs, ops, Pauli::identity(n), budget)?;
    let (weighted, peak_b, work_b) = propagate(
        inputs,
        ops,
        observable.clone(),
        Budget {
            max_work: budget
                .max_work
                .checked_sub(work_a)
                .filter(|w| *w > 0)
                .ok_or_else(|| invalid("Pauli propagation work budget exhausted"))?,
            ..budget
        },
    )?;
    if !accept.is_finite()
        || !weighted.is_finite()
        || !(-1e-12..=1.0 + 1e-12).contains(&accept)
        || weighted.abs() > accept + 1e-10
    {
        return Err(invalid("numerical invariant violated"));
    }
    let accept = accept.clamp(0.0, 1.0);
    // Below this threshold conditional expectations are ill conditioned in f64.
    if accept > 0.0 && accept < 1e-12 {
        return Err(invalid(
            "selected probability too small for stable f64 normalization",
        ));
    }
    Ok(PropagationResult {
        acceptance_probability: accept,
        weighted_expectation: weighted,
        conditional_expectation: if accept == 0.0 {
            None
        } else {
            Some((weighted / accept).clamp(-1.0, 1.0))
        },
        peak_terms: peak_a.max(peak_b),
        work: validation_work + work_a + work_b,
    })
}
