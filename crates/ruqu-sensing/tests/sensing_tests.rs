//! Integration tests for `ruqu-sensing`: thresholding, correlated-vs-isolated
//! fragility, weakest-component identification, and fault-field gating.
//!
//! All diagnoses use fixed seeds and small topologies (≤ 25 qubits) to stay
//! deterministic and within the syndrome-diagnosis budget.

use ruqu_possibility::CoherenceGate;
use ruqu_sensing::{fault_field, syndromes_from_samples, SensorChannel, SystemTopology};

fn labels(ids: &[&str]) -> Vec<String> {
    ids.iter().map(|s| s.to_string()).collect()
}

#[test]
fn thresholding_produces_correct_detector_bits() {
    let channels = vec![
        SensorChannel::new("a", 0.5),
        SensorChannel::new("b", 0.5),
        SensorChannel::new("c", 0.5),
    ];
    let samples = vec![
        (10, vec![0.9, 0.1, 0.6]), // a, c fire
        (20, vec![0.4, 0.4, 0.4]), // none fire
        (30, vec![0.51, 0.99, 0.50]), // a, b fire (c == threshold, not >)
    ];

    let syn = syndromes_from_samples(&channels, &samples).unwrap();
    assert_eq!(syn.len(), 3);
    assert_eq!(syn[0].detector_bits, vec![true, false, true]);
    assert_eq!(syn[1].detector_bits, vec![false, false, false]);
    assert_eq!(syn[2].detector_bits, vec![true, true, false]);

    assert_eq!(syn[0].timestamp_ns, 10);
    for s in &syn {
        assert!((0.0..=1.0).contains(&s.confidence));
    }
    // Unanimous-quiet sample has high agreement.
    assert!(syn[1].confidence > 0.0);
}

#[test]
fn mismatched_sample_width_is_an_error() {
    let channels = vec![SensorChannel::new("a", 0.5), SensorChannel::new("b", 0.5)];
    let samples = vec![(1, vec![0.9])];
    assert!(syndromes_from_samples(&channels, &samples).is_err());
}

#[test]
fn correlated_failures_are_more_severe_than_isolated_noise() {
    // 4 components in a small chain (4 comps + 3 conns = 7 qubits).
    let comps = labels(&["w", "x", "y", "z"]);
    let conns = vec![(0, 1), (1, 2), (2, 3)];

    // CORRELATED: components y and z are unhealthy and jointly stressed -> high
    // fault rate driving the shared graph.
    let correlated = SystemTopology::new(comps.clone(), conns.clone())
        .with_health(2, 0.2)
        .with_health(3, 0.2);
    let corr_diag = correlated.diagnose(0.6, 40, 7).unwrap();

    // ISOLATED: healthy system, one component flickers occasionally (low rate).
    let isolated = SystemTopology::new(comps.clone(), conns.clone());
    let iso_diag = isolated.diagnose(0.05, 40, 7).unwrap();

    assert!(
        corr_diag.severity > iso_diag.severity,
        "correlated failures (severity {}) should exceed isolated noise (severity {})",
        corr_diag.severity,
        iso_diag.severity
    );
    assert_eq!(corr_diag.fragility_scores.len(), 4);
}

#[test]
fn high_fault_rate_identifies_a_weakest_component() {
    let comps = labels(&["a", "b", "c", "d", "e"]);
    // 5 comps + 5 conns = 10 qubits.
    let conns = vec![(0, 1), (1, 2), (2, 3), (3, 4), (0, 4)];
    let topo = SystemTopology::new(comps, conns);
    assert!(topo.qubit_budget() <= 25);

    let diag = topo.diagnose(0.5, 60, 4242).unwrap();
    let weakest = diag.weakest_component.expect("a weakest component");
    assert!(diag.severity > 0.0);

    // The reported weakest must be the (a) component with the top fragility.
    let max = diag
        .fragility_scores
        .iter()
        .cloned()
        .fold(("".to_string(), f64::MIN), |acc, x| if x.1 > acc.1 { x } else { acc });
    assert_eq!(weakest, max.0);
    assert!((diag.severity - max.1).abs() < 1e-12);
}

#[test]
fn diagnosis_is_deterministic_for_a_fixed_seed() {
    let comps = labels(&["a", "b", "c", "d"]);
    let conns = vec![(0, 1), (1, 2), (2, 3)];
    let topo = SystemTopology::new(comps, conns);

    let d1 = topo.diagnose(0.4, 30, 77).unwrap();
    let d2 = topo.diagnose(0.4, 30, 77).unwrap();
    assert_eq!(d1, d2);
}

#[test]
fn fault_field_builds_a_gateable_collapsible_field() {
    let channels = vec![
        SensorChannel::new("comp_a", 0.5),
        SensorChannel::new("comp_b", 0.5),
        SensorChannel::new("comp_c", 0.5),
    ];
    // comp_a and comp_b fail together repeatedly; comp_c is quiet.
    let samples = vec![
        (1, vec![0.9, 0.9, 0.1]),
        (2, vec![0.8, 0.85, 0.2]),
        (3, vec![0.95, 0.92, 0.0]),
        (4, vec![0.1, 0.1, 0.1]),
    ];
    let syndromes = syndromes_from_samples(&channels, &samples).unwrap();

    let comp_labels = labels(&["comp_a", "comp_b", "comp_c"]);
    let field = fault_field(&syndromes, &comp_labels);

    assert!(!field.is_empty());
    assert_eq!(field.len(), 3);
    assert!(field.total_power() > 0.0);

    // The field can be gated and collapsed to a receipt.
    let _decision = CoherenceGate::with_defaults().evaluate(&field);
    let (selected, receipt) = field.collapse(2026).unwrap();
    assert_eq!(receipt.selected_id, selected.id);
    assert!(comp_labels.contains(&receipt.selected_id));

    // Jointly-failing components carry more amplitude than the quiet one, so
    // argmax should prefer one of them.
    let top = field.argmax().unwrap();
    assert!(top.id == "comp_a" || top.id == "comp_b");
}

#[test]
fn fault_field_empty_labels_yields_empty_field() {
    let field = fault_field(&[], &[]);
    assert!(field.is_empty());
}
