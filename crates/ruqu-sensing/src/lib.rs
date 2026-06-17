//! # ruqu-sensing — Telemetry to syndrome streams (ADR-258 §11)
//!
//! Structural anomaly detection: convert raw telemetry into *syndrome streams*
//! and feed them to QEC-style syndrome diagnosis to localize fragile system
//! components.
//!
//! The pipeline has three stages:
//!
//! 1. **Sensing.** A bank of [`SensorChannel`]s thresholds a time-series of
//!    samples into a [`SensorSyndrome`] per timestamp — a detector-bit vector
//!    where bit *i* fires when channel *i* exceeds its threshold, plus a
//!    [`confidence`](SensorSyndrome::confidence) derived from the signal margin
//!    and inter-channel agreement.
//! 2. **Diagnosis.** A [`SystemTopology`] of named components and connections
//!    drives [`ruqu_exotic::syndrome_diagnosis`], producing per-component
//!    fragility scores, a weakest component, fault propagators, and an overall
//!    [`severity`](SystemDiagnosis::severity).
//! 3. **Explanation.** [`fault_field`] turns a batch of syndromes into a
//!    [`PossibilityField`] of candidate root-cause components, so callers can
//!    gate and collapse to a most-likely explanation with a replayable receipt.
//!
//! ## Qubit budget
//!
//! Syndrome diagnosis maps `components + connections` onto qubits and is capped
//! at 25 total. Keep topologies small (≈4-6 components, ≤10 connections).
//!
//! ## Example
//!
//! ```
//! use ruqu_sensing::{SensorChannel, syndromes_from_samples, SystemTopology};
//!
//! let channels = vec![
//!     SensorChannel::new("cpu", 0.8),
//!     SensorChannel::new("mem", 0.8),
//! ];
//! // (timestamp_ns, [per-channel value])
//! let samples = vec![
//!     (1, vec![0.9, 0.2]), // cpu fires
//!     (2, vec![0.95, 0.9]), // both fire
//! ];
//! let syndromes = syndromes_from_samples(&channels, &samples).unwrap();
//! assert_eq!(syndromes[0].detector_bits, vec![true, false]);
//! ```

use anyhow::{bail, Result};
use ruqu_exotic::syndrome_diagnosis::{
    Component, Connection, DiagnosisConfig, SystemDiagnostics,
};
use ruqu_possibility::{Possibility, PossibilityField};
use serde::{Deserialize, Serialize};

/// A detector-bit pattern extracted from telemetry at one instant.
///
/// `detector_bits[i]` is `true` when channel *i* tripped its threshold. This is
/// the structural analogue of a QEC syndrome: a parity/anomaly pattern that
/// localizes where something went wrong.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SensorSyndrome {
    /// Logical origin of this syndrome (sensor bank / subsystem id).
    pub source: String,
    /// One bit per channel; `true` = channel exceeded its threshold.
    pub detector_bits: Vec<bool>,
    /// Confidence in `[0, 1]` from signal margin and inter-channel agreement.
    pub confidence: f64,
    /// Sample timestamp in nanoseconds.
    pub timestamp_ns: u64,
}

impl SensorSyndrome {
    /// Number of fired detector bits.
    pub fn fired_count(&self) -> usize {
        self.detector_bits.iter().filter(|&&b| b).count()
    }

    /// Whether any detector bit fired.
    pub fn any_fired(&self) -> bool {
        self.detector_bits.iter().any(|&b| b)
    }
}

/// One thresholding sensor channel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SensorChannel {
    /// Channel identifier (maps to a detector-bit index).
    pub id: String,
    /// A sample value strictly greater than this fires the channel's bit.
    pub threshold: f64,
}

impl SensorChannel {
    /// Construct a channel.
    pub fn new(id: impl Into<String>, threshold: f64) -> Self {
        Self {
            id: id.into(),
            threshold,
        }
    }

    /// Whether `value` trips this channel.
    pub fn fires(&self, value: f64) -> bool {
        value > self.threshold
    }
}

/// Turn a stream of per-timestamp samples into one [`SensorSyndrome`] each.
///
/// Each sample is `(timestamp_ns, values)` where `values[i]` is the reading of
/// channel `channels[i]`. Bit *i* fires when `values[i] > channels[i].threshold`.
///
/// The `confidence` of a syndrome combines two signals, each in `[0, 1]`:
///
/// * **margin** — the mean absolute distance of every reading from its
///   threshold, squashed so clearly-separated readings approach 1 and
///   readings hovering on the threshold approach 0;
/// * **agreement** — how unanimous the channels are (all-fired or all-quiet is
///   high agreement; a 50/50 split is low).
///
/// The two are averaged. Empty channel banks yield `confidence = 0`.
///
/// # Errors
/// Returns an error if any sample's value count does not match `channels`.
pub fn syndromes_from_samples(
    channels: &[SensorChannel],
    samples: &[(u64, Vec<f64>)],
) -> Result<Vec<SensorSyndrome>> {
    let source = channel_bank_label(channels);
    let mut out = Vec::with_capacity(samples.len());
    for (idx, (ts, values)) in samples.iter().enumerate() {
        if values.len() != channels.len() {
            bail!(
                "sample {idx} has {} values but there are {} channels",
                values.len(),
                channels.len()
            );
        }
        out.push(syndrome_at(&source, channels, *ts, values));
    }
    Ok(out)
}

/// Build a single [`SensorSyndrome`] for one timestamp.
fn syndrome_at(
    source: &str,
    channels: &[SensorChannel],
    timestamp_ns: u64,
    values: &[f64],
) -> SensorSyndrome {
    let detector_bits: Vec<bool> = channels
        .iter()
        .zip(values)
        .map(|(ch, &v)| ch.fires(v))
        .collect();

    let confidence = confidence_of(channels, values, &detector_bits);

    SensorSyndrome {
        source: source.to_string(),
        detector_bits,
        confidence,
        timestamp_ns,
    }
}

/// Confidence from signal margin and inter-channel agreement, each in `[0, 1]`.
fn confidence_of(channels: &[SensorChannel], values: &[f64], bits: &[bool]) -> f64 {
    let n = channels.len();
    if n == 0 {
        return 0.0;
    }

    // Mean absolute margin, squashed: tanh keeps it bounded and monotone, so a
    // reading far from its threshold contributes near-1 confidence.
    let mean_margin: f64 = channels
        .iter()
        .zip(values)
        .map(|(ch, &v)| (v - ch.threshold).abs())
        .sum::<f64>()
        / n as f64;
    let margin = mean_margin.tanh();

    // Agreement: a tent function — 1 when unanimous (all fired or all quiet),
    // 0 at a perfect 50/50 split.
    let fired = bits.iter().filter(|&&b| b).count() as f64;
    let frac = fired / n as f64;
    let agreement = (1.0 - 2.0 * (frac - 0.5).abs()).clamp(0.0, 1.0);

    ((margin + agreement) / 2.0).clamp(0.0, 1.0)
}

/// Stable label for a bank of channels, used as a syndrome `source`.
fn channel_bank_label(channels: &[SensorChannel]) -> String {
    if channels.is_empty() {
        return "sensor_bank[]".to_string();
    }
    let ids: Vec<&str> = channels.iter().map(|c| c.id.as_str()).collect();
    format!("sensor_bank[{}]", ids.join(","))
}

/// Aggregated result of a structural diagnosis run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SystemDiagnosis {
    /// Per-component fragility score (syndrome appearances / rounds).
    pub fragility_scores: Vec<(String, f64)>,
    /// The most fragile component, if any.
    pub weakest_component: Option<String>,
    /// Components that propagate faults beyond where they were directly injected.
    pub fault_propagators: Vec<String>,
    /// Overall severity: the maximum fragility score across components.
    pub severity: f64,
}

/// A named system graph that drives syndrome diagnosis.
///
/// Wraps [`ruqu_exotic::syndrome_diagnosis`]: components become data qubits and
/// connections become parity-check ancillas, so `components + connections` must
/// be `≤ 25`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SystemTopology {
    /// Component labels (each becomes a data qubit).
    pub components: Vec<String>,
    /// Per-component health in `[0, 1]` (1 = healthy). Defaults to all-healthy.
    pub health: Vec<f64>,
    /// Directed component-index pairs with coupling strength.
    pub connections: Vec<(usize, usize, f64)>,
}

impl SystemTopology {
    /// Build an all-healthy topology from component labels and `(from, to)`
    /// index pairs (unit coupling strength).
    pub fn new(components: Vec<String>, connections: Vec<(usize, usize)>) -> Self {
        let health = vec![1.0; components.len()];
        let connections = connections
            .into_iter()
            .map(|(f, t)| (f, t, 1.0))
            .collect();
        Self {
            components,
            health,
            connections,
        }
    }

    /// Override the health of a component by index (1 = healthy, 0 = failed).
    pub fn with_health(mut self, index: usize, health: f64) -> Self {
        if let Some(h) = self.health.get_mut(index) {
            *h = health.clamp(0.0, 1.0);
        }
        self
    }

    /// Total qubits required (`components + connections`); must be `≤ 25`.
    pub fn qubit_budget(&self) -> usize {
        self.components.len() + self.connections.len()
    }

    /// Run syndrome diagnosis and aggregate into a [`SystemDiagnosis`].
    ///
    /// # Errors
    /// Propagates the underlying quantum errors (notably the 25-qubit limit) as
    /// `anyhow::Error`.
    pub fn diagnose(
        &self,
        fault_injection_rate: f64,
        num_rounds: usize,
        seed: u64,
    ) -> Result<SystemDiagnosis> {
        let components: Vec<Component> = self
            .components
            .iter()
            .enumerate()
            .map(|(i, id)| Component {
                id: id.clone(),
                health: self.health.get(i).copied().unwrap_or(1.0),
            })
            .collect();

        let connections: Vec<Connection> = self
            .connections
            .iter()
            .map(|&(from, to, strength)| Connection { from, to, strength })
            .collect();

        let diag = SystemDiagnostics::new(components, connections);
        let config = DiagnosisConfig {
            fault_injection_rate,
            num_rounds,
            seed,
        };

        let result = diag
            .diagnose(&config)
            .map_err(|e| anyhow::anyhow!("syndrome diagnosis failed: {e}"))?;

        let severity = result
            .fragility_scores
            .iter()
            .map(|(_, s)| *s)
            .fold(0.0_f64, f64::max);

        Ok(SystemDiagnosis {
            fragility_scores: result.fragility_scores,
            weakest_component: result.weakest_component,
            fault_propagators: result.fault_propagators,
            severity,
        })
    }
}

/// Build a [`PossibilityField`] of candidate root-cause components from a batch
/// of syndromes, so callers can gate and collapse to a most-likely explanation.
///
/// For each component label (detector-bit index *i*):
///
/// * **amplitude** grows with how often bit *i* fired, weighted by each
///   syndrome's confidence — components that are anomalous across many
///   high-confidence samples carry more weight;
/// * **phase** encodes correlation with the rest of the field. A component
///   whose firing tracks the global anomaly level gets phase ≈ 0 (it
///   interferes *constructively* — a coherent, jointly-failing cluster), while
///   a component that fires when others are quiet gets phase ≈ π
///   (*anti-correlated*, destructive — isolated noise that should not dominate
///   a collapse).
///
/// Components that never fire still appear with a small floor amplitude so the
/// field is non-empty and every candidate is representable. Bit indices beyond
/// `component_labels` are ignored; missing labels are skipped.
pub fn fault_field(
    syndromes: &[SensorSyndrome],
    component_labels: &[String],
) -> PossibilityField<String> {
    let n = component_labels.len();
    if n == 0 {
        return PossibilityField::new(Vec::new());
    }

    // Per-component weighted fire mass and the global per-sample fired fraction.
    let mut fire_mass = vec![0.0_f64; n];
    let mut sample_frac = Vec::with_capacity(syndromes.len());
    for s in syndromes {
        let width = s.detector_bits.len().max(1) as f64;
        let frac = s.fired_count() as f64 / width;
        sample_frac.push(frac);
        let w = s.confidence.clamp(0.0, 1.0).max(0.05); // floor so zero-conf still counts a little
        for (i, mass) in fire_mass.iter_mut().enumerate() {
            if s.detector_bits.get(i).copied().unwrap_or(false) {
                *mass += w;
            }
        }
    }

    let total_samples = syndromes.len().max(1) as f64;
    let mean_global_frac: f64 = if sample_frac.is_empty() {
        0.0
    } else {
        sample_frac.iter().sum::<f64>() / sample_frac.len() as f64
    };

    let mut candidates = Vec::with_capacity(n);
    for (i, label) in component_labels.iter().enumerate() {
        // Amplitude: normalized accumulated mass, with a small floor.
        let amplitude = (fire_mass[i] / total_samples).sqrt() + 0.05;

        // Correlation of this component's firing with the global anomaly level.
        // Positive => fails together with the field (constructive, phase 0).
        // Negative => fires when others are quiet (anti-correlated, phase ~pi).
        let corr = correlation_with_global(syndromes, i, mean_global_frac, &sample_frac);
        // Map correlation in [-1, 1] to phase in [pi, 0]: corr=+1 -> 0, corr=-1 -> pi.
        let phase = std::f64::consts::PI * (1.0 - corr) / 2.0;

        candidates.push(Possibility::new(label.clone(), label.clone(), amplitude, phase));
    }

    PossibilityField::new(candidates)
}

/// Pearson-style correlation between component `i`'s firing indicator and the
/// per-sample global fired fraction, in `[-1, 1]`. Falls back to 0 when either
/// series has no variance (e.g. a component that always/never fires).
fn correlation_with_global(
    syndromes: &[SensorSyndrome],
    i: usize,
    mean_global: f64,
    sample_frac: &[f64],
) -> f64 {
    let n = syndromes.len();
    if n == 0 {
        return 0.0;
    }
    let xs: Vec<f64> = syndromes
        .iter()
        .map(|s| if s.detector_bits.get(i).copied().unwrap_or(false) { 1.0 } else { 0.0 })
        .collect();
    let mean_x = xs.iter().sum::<f64>() / n as f64;

    let mut cov = 0.0;
    let mut var_x = 0.0;
    let mut var_y = 0.0;
    for (k, &x) in xs.iter().enumerate() {
        let dx = x - mean_x;
        let dy = sample_frac.get(k).copied().unwrap_or(0.0) - mean_global;
        cov += dx * dy;
        var_x += dx * dx;
        var_y += dy * dy;
    }
    if var_x <= 1e-12 || var_y <= 1e-12 {
        return 0.0;
    }
    (cov / (var_x.sqrt() * var_y.sqrt())).clamp(-1.0, 1.0)
}

/// Crate version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
