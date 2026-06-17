//! Acceptance-style tests for the possibility runtime foundation.
//!
//! Covers ADR-258 §22 items that live at this layer: collapse determinism,
//! replay reproducibility (Test 6), and receipt hashing; plus the structural
//! quantities (entropy, coherence) the gate depends on.

use std::f64::consts::PI;

use ruqu_possibility::{
    CoherenceGate, CollapseReceipt, EvidenceReceipt, GateDecision, GateThresholds, Possibility,
    PossibilityField,
};

fn field() -> PossibilityField<String> {
    PossibilityField::new(vec![
        Possibility::new("a", "alpha".to_string(), 0.9, 0.0),
        Possibility::new("b", "beta".to_string(), 0.4, 0.1),
        Possibility::new("c", "gamma".to_string(), 0.2, PI),
    ])
}

#[test]
fn entropy_is_zero_for_pure_field_and_maximal_for_uniform() {
    let pure = PossibilityField::new(vec![Possibility::new("only", (), 1.0, 0.0)]);
    assert!(pure.entropy().abs() < 1e-12);

    let uniform = PossibilityField::new(vec![
        Possibility::new("a", (), 0.5, 0.0),
        Possibility::new("b", (), 0.5, 0.0),
        Possibility::new("c", (), 0.5, 0.0),
        Possibility::new("d", (), 0.5, 0.0),
    ]);
    // 4 equal candidates -> log2(4) = 2 bits.
    assert!((uniform.entropy() - 2.0).abs() < 1e-9);
}

#[test]
fn coherence_is_one_when_phases_aligned_and_low_when_opposed() {
    let aligned = PossibilityField::new(vec![
        Possibility::new("a", (), 0.7, 0.3),
        Possibility::new("b", (), 0.5, 0.3),
    ]);
    assert!((aligned.coherence() - 1.0).abs() < 1e-9);

    let opposed = PossibilityField::new(vec![
        Possibility::new("a", (), 1.0, 0.0),
        Possibility::new("b", (), 1.0, PI),
    ]);
    assert!(opposed.coherence() < 1e-9);
}

#[test]
fn collapse_is_deterministic_for_a_given_seed() {
    let f = field();
    let (s1, _) = f.collapse(7).unwrap();
    let (s2, _) = f.collapse(7).unwrap();
    assert_eq!(s1.id, s2.id);
}

#[test]
fn replay_reproduces_selection_and_coherence_within_tolerance() {
    // ADR-258 §22 Test 6: same receipt + same seed -> same selection and
    // coherence within 1e-9.
    let f = field();
    let (first, r1) = f.collapse(123).unwrap();
    let (second, r2) = f.collapse(r1.seed).unwrap();
    assert_eq!(first.id, second.id);
    assert!((r1.coherence - r2.coherence).abs() < 1e-9);
    assert_eq!(r1.field_hash, r2.field_hash);
    assert_eq!(r1.run_id, r2.run_id);
}

#[test]
fn collapse_argmax_picks_highest_probability_regardless_of_seed() {
    let f = field(); // "a" has the largest amplitude (0.9)
    let (s_a, r_a) = f.collapse_argmax(1).unwrap();
    let (s_b, r_b) = f.collapse_argmax(999).unwrap();
    assert_eq!(s_a.id, "a");
    assert_eq!(s_b.id, "a");
    assert_eq!(r_a.selected_id, "a");
    // Selection is seed-independent; metrics describe the natural-order field.
    assert_eq!(r_a.field_hash, f.field_hash());
    assert_eq!(r_a.field_hash, r_b.field_hash);
    assert_eq!(r_a.rejected.len(), f.len() - 1);
}

#[test]
fn receipt_hash_is_stable_and_content_addressed() {
    let f = field();
    let (_, receipt) = f.collapse(42).unwrap();
    let h1 = receipt.receipt_hash();
    let h2 = receipt.receipt_hash();
    assert_eq!(h1, h2);
    assert_eq!(h1.len(), 64); // BLAKE3 hex

    // Round-trips through JSON without changing content hash.
    let json = receipt.to_json();
    let decoded: CollapseReceipt = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.receipt_hash(), h1);

    // A different field produces a different field hash (and thus receipt).
    let other = PossibilityField::new(vec![Possibility::new("z", "zeta".to_string(), 1.0, 0.0)]);
    let (_, other_receipt) = other.collapse(42).unwrap();
    assert_ne!(other_receipt.field_hash, receipt.field_hash);
}

#[test]
fn rejected_candidates_are_recorded_with_reasons() {
    let f = field();
    let (selected, receipt) = f.collapse(1).unwrap();
    assert_eq!(receipt.selected_id, selected.id);
    assert_eq!(receipt.rejected.len(), f.len() - 1);
    assert!(receipt.rejected.iter().all(|r| !r.reason.is_empty()));
}

#[test]
fn gate_permits_coherent_low_entropy_and_denies_contradictory() {
    let gate = CoherenceGate::with_defaults();

    let coherent = PossibilityField::new(vec![
        Possibility::new("a", (), 0.95, 0.0),
        Possibility::new("b", (), 0.10, 0.0),
    ]);
    assert_eq!(gate.evaluate(&coherent), GateDecision::Permit);

    let contradictory = PossibilityField::new(vec![
        Possibility::new("a", (), 1.0, 0.0),
        Possibility::new("b", (), 0.95, PI),
    ]);
    assert_eq!(gate.evaluate(&contradictory), GateDecision::Deny);
}

#[test]
fn gate_defers_on_high_entropy_even_if_coherent() {
    // Many equal, phase-aligned candidates: coherence high, entropy high.
    let candidates = (0..16)
        .map(|i| Possibility::new(format!("c{i}"), (), 0.5, 0.0))
        .collect();
    let field = PossibilityField::new(candidates);
    let gate = CoherenceGate::new(GateThresholds {
        permit_coherence: 0.7,
        deny_coherence: 0.3,
        max_entropy: 2.0,
    });
    // entropy = log2(16) = 4 bits > 2.0 -> DEFER despite coherence 1.0.
    assert!((field.coherence() - 1.0).abs() < 1e-9);
    assert_eq!(gate.evaluate(&field), GateDecision::Defer);
}

#[test]
fn normalize_makes_total_power_unit() {
    let mut f = field();
    f.normalize();
    assert!((f.total_power() - 1.0).abs() < 1e-9);
}

#[test]
fn empty_field_cannot_collapse() {
    let empty: PossibilityField<()> = PossibilityField::new(vec![]);
    assert!(empty.collapse(0).is_err());
}

#[test]
fn evidence_receipt_hashes_payload() {
    let e = EvidenceReceipt::new("doc-1", 0.9, b"some source text");
    assert_eq!(e.payload_hash.len(), 64);
    let e2 = EvidenceReceipt::new("doc-1", 0.9, b"some source text");
    assert_eq!(e.payload_hash, e2.payload_hash);
}
