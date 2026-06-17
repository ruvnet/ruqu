//! Integration tests for `ruqu-receipts`: hash-chain integrity, JSONL
//! round-trip, and replay verification (ADR-258 §22 Test 6).

use ruqu_possibility::{Possibility, PossibilityField};
use ruqu_receipts::{verify_replay, ReceiptStore, GENESIS_PREV_HASH};

/// A small, coherent field that collapses deterministically.
fn small_field(tag: &str) -> PossibilityField<String> {
    PossibilityField::new(vec![
        Possibility::new(format!("{tag}_a"), format!("answer a for {tag}"), 0.9, 0.0),
        Possibility::new(format!("{tag}_b"), format!("answer b for {tag}"), 0.4, 0.0),
        Possibility::new(format!("{tag}_c"), format!("answer c for {tag}"), 0.2, 0.0),
    ])
}

#[test]
fn appended_chain_verifies() {
    let mut store = ReceiptStore::new();
    assert!(store.is_empty());
    assert_eq!(store.tip_hash(), GENESIS_PREV_HASH);

    for i in 0..5u64 {
        let (_sel, receipt) = small_field(&format!("f{i}")).collapse(i).unwrap();
        store.append(receipt);
    }

    assert_eq!(store.len(), 5);
    assert!(!store.is_empty());
    assert!(store.verify_chain());

    // Genesis entry links to all-zero prev hash; sequence numbers are dense.
    assert_eq!(store.entries[0].prev_hash, GENESIS_PREV_HASH);
    for (i, e) in store.entries.iter().enumerate() {
        assert_eq!(e.seq, i as u64);
    }
    // Each entry links to the previous entry's hash.
    for w in store.entries.windows(2) {
        assert_eq!(w[1].prev_hash, w[0].entry_hash);
    }
    assert_eq!(store.tip_hash(), store.entries.last().unwrap().entry_hash);
}

#[test]
fn mutating_a_stored_receipt_breaks_the_chain() {
    let mut store = ReceiptStore::new();
    for i in 0..4u64 {
        let (_sel, receipt) = small_field(&format!("f{i}")).collapse(i).unwrap();
        store.append(receipt);
    }
    assert!(store.verify_chain());

    // Tamper with a receipt's recorded field/decision without recomputing hashes.
    store.entries[1].receipt.selected_id = "forged".to_string();
    store.entries[2].receipt.coherence = 0.123456;

    assert!(
        !store.verify_chain(),
        "tampering with a stored receipt must be detected"
    );
}

#[test]
fn jsonl_round_trip_preserves_chain() {
    let mut store = ReceiptStore::new();
    for i in 0..6u64 {
        let (_sel, receipt) = small_field(&format!("f{i}")).collapse(i * 7 + 1).unwrap();
        store.append(receipt);
    }
    assert!(store.verify_chain());

    let jsonl = store.to_jsonl();
    // One JSON object per line.
    assert_eq!(jsonl.lines().count(), 6);

    let restored = ReceiptStore::from_jsonl(&jsonl).unwrap();
    assert_eq!(restored, store);
    assert!(restored.verify_chain());

    // Blank lines are tolerated.
    let padded = format!("\n{jsonl}\n\n");
    let restored2 = ReceiptStore::from_jsonl(&padded).unwrap();
    assert_eq!(restored2.len(), 6);
    assert!(restored2.verify_chain());
}

#[test]
fn jsonl_round_trip_detects_post_load_tampering() {
    let mut store = ReceiptStore::new();
    let (_sel, receipt) = small_field("x").collapse(99).unwrap();
    store.append(receipt);

    let jsonl = store.to_jsonl();
    let mut restored = ReceiptStore::from_jsonl(&jsonl).unwrap();
    assert!(restored.verify_chain());

    restored.entries[0].receipt.seed ^= 0xDEAD_BEEF;
    assert!(!restored.verify_chain());
}

#[test]
fn replay_verifies_untouched_field_and_rejects_modified() {
    let field = small_field("replay");
    let (_sel, receipt) = field.collapse(2024).unwrap();

    // Untouched field reproduces exactly.
    let good = verify_replay(&receipt, &field);
    assert!(good.reproduced);
    assert!(good.selected_matches);
    assert!(good.coherence_delta < 1e-9);

    // Modified field: shift amplitudes and one phase so coherence/selection move.
    let mut modified = field.clone();
    modified.candidates[0].amplitude = 0.1;
    modified.candidates[1].amplitude = 0.95;
    modified.candidates[2].phase = std::f64::consts::PI;
    let bad = verify_replay(&receipt, &modified);
    assert!(!bad.reproduced, "a modified field must not replay-reproduce");
}

#[test]
fn adr_test_6_same_field_and_seed_reproduces_selection_and_coherence() {
    // ADR-258 §22 Test 6: replay determinism within 1e-9.
    let field = small_field("adr6");
    let seed = 0xC0FFEE;

    let (sel1, r1) = field.collapse(seed).unwrap();
    let (sel2, r2) = field.collapse(seed).unwrap();

    assert_eq!(sel1.id, sel2.id);
    assert_eq!(r1.selected_id, r2.selected_id);
    assert!((r1.coherence - r2.coherence).abs() < 1e-9);

    let verdict = verify_replay(&r1, &field);
    assert!(verdict.reproduced);
    assert_eq!(verdict.coherence_delta, 0.0);
}
