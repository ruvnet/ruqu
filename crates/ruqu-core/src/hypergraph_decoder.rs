//! Bounded most likely error decoding for detector hypergraphs.
//!
//! Inspired by the search formulation in Tesseract (arXiv:2503.10988), this is
//! an independent, small reference implementation, not Tesseract's A* decoder.
//! Each mechanism is an independent Bernoulli event; one event can flip any
//! number of detectors and logical observables. XOR combines events. Correlation
//! *within* an event is supported, but dependencies *between* events are not.
//!
//! Search minimizes `sum(selected_i * log((1-p_i)/p_i))`, equivalent to maximizing
//! an individual error configuration's probability. It does not sum probability
//! over a logical coset and therefore is not maximum likelihood logical decoding.
//! All optimality statements are with respect to the computed f64 costs.
//!
//! Bounds: at most 64 mechanisms, 128 detectors and 128 logical observables.
//! Worst case time is O(2^n), auxiliary memory O(n), recursion depth n + 1.
//! The mandatory node budget bounds work; an incomplete search returns an error,
//! even if it has found a feasible candidate. No beam or heuristic cutoff silently
//! changes an exact result into an approximate one.

use std::fmt;

/// An independently activated fault, including correlated detector flips.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ErrorMechanism {
    pub detectors: u128,
    pub logicals: u128,
    pub probability: f64,
}

/// Validated detector error model.
#[derive(Clone, Debug)]
pub struct HypergraphDecoder {
    detector_count: u32,
    mechanisms: Vec<ErrorMechanism>,
    weights: Vec<f64>,
    suffix_detectors: Vec<u128>,
    baseline_selection: u64,
    baseline_syndrome: u128,
    baseline_cost: f64,
}

/// An optimal individual configuration after a completed search.
#[derive(Clone, Debug, PartialEq)]
pub struct DecodedError {
    /// Bit i selects the input model's ith mechanism.
    pub selected_mechanisms: u64,
    pub logical_mask: u128,
    /// Sum of selected mechanisms' log odds, relative to selecting no errors.
    pub log_odds_cost: f64,
    pub visited_nodes: u64,
}

/// Input failures and explicit search completion states.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HypergraphDecodeError {
    TooManyMechanisms,
    InvalidBitCount,
    InvalidProbability {
        mechanism: usize,
    },
    DetectorMaskOutOfRange {
        mechanism: usize,
    },
    LogicalMaskOutOfRange {
        mechanism: usize,
    },
    SyndromeOutOfRange,
    ZeroNodeBudget,
    /// Complete search proves that no configuration produces the syndrome.
    NoSolution {
        visited_nodes: u64,
    },
    /// Optimality was not proven. A feasible incumbent is intentionally withheld.
    BudgetExceeded {
        visited_nodes: u64,
    },
}

impl fmt::Display for HypergraphDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "hypergraph decoding: {self:?}")
    }
}

impl std::error::Error for HypergraphDecodeError {}

fn fits(mask: u128, count: u32) -> bool {
    count == 128 || mask >> count == 0
}

impl HypergraphDecoder {
    /// Validate a bounded model. Probabilities must be finite and strictly
    /// between zero and one; remove impossible events and fold deterministic
    /// events into the input syndrome before constructing this model.
    pub fn new(
        detector_count: u32,
        logical_count: u32,
        mechanisms: Vec<ErrorMechanism>,
    ) -> Result<Self, HypergraphDecodeError> {
        use HypergraphDecodeError::*;
        if detector_count > 128 || logical_count > 128 {
            return Err(InvalidBitCount);
        }
        if mechanisms.len() > 64 {
            return Err(TooManyMechanisms);
        }
        let mut weights = Vec::with_capacity(mechanisms.len());
        let mut baseline_selection = 0;
        let mut baseline_syndrome = 0;
        let mut baseline_cost = 0.0;
        for (i, mechanism) in mechanisms.iter().enumerate() {
            let p = mechanism.probability;
            if !p.is_finite() || p <= 0.0 || p >= 1.0 {
                return Err(InvalidProbability { mechanism: i });
            }
            if !fits(mechanism.detectors, detector_count) {
                return Err(DetectorMaskOutOfRange { mechanism: i });
            }
            if !fits(mechanism.logicals, logical_count) {
                return Err(LogicalMaskOutOfRange { mechanism: i });
            }
            // Difference of logs avoids overflow of (1-p)/p for subnormal p.
            let log_odds = (-p).ln_1p() - p.ln();
            weights.push(log_odds.abs());
            if p > 0.5 {
                baseline_selection |= 1_u64 << i;
                baseline_syndrome ^= mechanism.detectors;
                baseline_cost += log_odds;
            }
        }
        let mut suffix_detectors = vec![0; mechanisms.len() + 1];
        for i in (0..mechanisms.len()).rev() {
            suffix_detectors[i] = suffix_detectors[i + 1] | mechanisms[i].detectors;
        }
        Ok(Self {
            detector_count,
            mechanisms,
            weights,
            suffix_detectors,
            baseline_selection,
            baseline_syndrome,
            baseline_cost,
        })
    }

    /// Find the minimum cost error producing `syndrome` within `max_nodes`.
    /// A node is charged before evaluating bounds or visiting its children.
    /// Ties retain the first configuration encountered deterministically.
    pub fn decode(
        &self,
        syndrome: u128,
        max_nodes: u64,
    ) -> Result<DecodedError, HypergraphDecodeError> {
        if !fits(syndrome, self.detector_count) {
            return Err(HypergraphDecodeError::SyndromeOutOfRange);
        }
        if max_nodes == 0 {
            return Err(HypergraphDecodeError::ZeroNodeBudget);
        }
        let mut search = Search {
            model: self,
            max_nodes,
            visited: 0,
            best: None,
        };
        search.visit(0, syndrome ^ self.baseline_syndrome, 0, 0.0)?;
        let (penalty, flips) = search.best.ok_or(HypergraphDecodeError::NoSolution {
            visited_nodes: search.visited,
        })?;
        let selected = flips ^ self.baseline_selection;
        let mut logical_mask = 0;
        for (i, mechanism) in self.mechanisms.iter().enumerate() {
            if selected & (1_u64 << i) != 0 {
                logical_mask ^= mechanism.logicals;
            }
        }
        Ok(DecodedError {
            selected_mechanisms: selected,
            logical_mask,
            log_odds_cost: self.baseline_cost + penalty,
            visited_nodes: search.visited,
        })
    }
}

struct Search<'a> {
    model: &'a HypergraphDecoder,
    max_nodes: u64,
    visited: u64,
    best: Option<(f64, u64)>,
}

impl Search<'_> {
    fn visit(
        &mut self,
        index: usize,
        residual: u128,
        flips: u64,
        cost: f64,
    ) -> Result<(), HypergraphDecodeError> {
        if self.visited == self.max_nodes {
            return Err(HypergraphDecodeError::BudgetExceeded {
                visited_nodes: self.visited,
            });
        }
        self.visited += 1;
        // Remaining penalties are nonnegative, including p > .5 after baseline
        // complementation. This bound cannot discard a lower cost completion.
        if self.best.map_or(false, |(best, _)| cost >= best) {
            return Ok(());
        }
        if residual & !self.model.suffix_detectors[index] != 0 {
            return Ok(());
        }
        if residual == 0 {
            self.best = Some((cost, flips));
            return Ok(());
        }
        if index == self.model.mechanisms.len() {
            return Ok(());
        }
        self.visit(index + 1, residual, flips, cost)?;
        self.visit(
            index + 1,
            residual ^ self.model.mechanisms[index].detectors,
            flips | (1_u64 << index),
            cost + self.model.weights[index],
        )
    }
}
