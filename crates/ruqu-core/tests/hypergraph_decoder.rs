use ruqu_core::hypergraph_decoder::{
    ErrorMechanism as E, HypergraphDecodeError as Err, HypergraphDecoder,
};

fn event(detectors: u128, logicals: u128, probability: f64) -> E {
    E {
        detectors,
        logicals,
        probability,
    }
}

fn enumerate(events: &[E], syndrome: u128) -> Option<(f64, u64)> {
    let mut best: Option<(f64, u64)> = None;
    for mask in 0..(1_u64 << events.len()) {
        let mut observed = 0;
        let mut probability = 1.0;
        for (i, e) in events.iter().enumerate() {
            let selected = mask & (1 << i) != 0;
            if selected {
                observed ^= e.detectors;
            }
            probability *= if selected {
                e.probability
            } else {
                1.0 - e.probability
            };
        }
        if observed == syndrome && best.map_or(true, |(p, _)| probability > p) {
            best = Some((probability, mask));
        }
    }
    best
}

#[test]
fn exhaustive_small_hypergraphs_match_independent_probability_oracle() {
    // Deterministic varied models, no RNG dependency. Includes probabilities
    // above .5 and hyperedges flipping up to four detectors simultaneously.
    for seed in 0..32_u128 {
        let events: Vec<_> = (0..8)
            .map(|i| {
                event(
                    (seed * 7 + i * 11 + 3) % 16,
                    (seed + i) % 4,
                    0.07 + ((seed + i * 3) % 9) as f64 * 0.1,
                )
            })
            .collect();
        let decoder = HypergraphDecoder::new(4, 2, events.clone()).unwrap();
        for syndrome in 0..16 {
            match enumerate(&events, syndrome) {
                None => assert!(matches!(
                    decoder.decode(syndrome, 1_000),
                    Err(Err::NoSolution { .. })
                )),
                Some((expected_probability, _)) => {
                    let actual = decoder.decode(syndrome, 1_000).unwrap();
                    let mut detectors = 0;
                    let mut logicals = 0;
                    let mut probability = 1.0;
                    let mut cost = 0.0;
                    for (i, e) in events.iter().enumerate() {
                        if actual.selected_mechanisms & (1 << i) != 0 {
                            detectors ^= e.detectors;
                            logicals ^= e.logicals;
                            probability *= e.probability;
                            cost += ((1.0 - e.probability) / e.probability).ln();
                        } else {
                            probability *= 1.0 - e.probability;
                        }
                    }
                    assert_eq!(detectors, syndrome);
                    assert_eq!(logicals, actual.logical_mask);
                    assert!((probability - expected_probability).abs() < 1e-12);
                    assert!((cost - actual.log_odds_cost).abs() < 1e-12);
                }
            }
        }
    }
}

#[test]
fn correlated_fault_and_logical_xor_are_preserved() {
    let decoder = HypergraphDecoder::new(
        4,
        2,
        vec![
            event(0b111, 0b11, 0.02),
            event(0b1000, 0b01, 0.03),
            event(1, 0, 0.001),
            event(0b110, 0, 0.001),
        ],
    )
    .unwrap();
    let result = decoder.decode(0b1111, 1_000).unwrap();
    assert_eq!(result.selected_mechanisms, 3);
    assert_eq!(result.logical_mask, 0b10);
}

#[test]
fn high_probability_and_undetectable_faults_use_complement_baseline() {
    let decoder = HypergraphDecoder::new(1, 1, vec![event(1, 0, 0.9), event(0, 1, 0.8)]).unwrap();
    assert_eq!(decoder.decode(1, 20).unwrap().selected_mechanisms, 3);
    let result = decoder.decode(0, 20).unwrap();
    assert_eq!(result.selected_mechanisms, 2);
    assert_eq!(result.logical_mask, 1);
}

#[test]
fn exhausted_search_and_budget_exhaustion_are_distinct() {
    let decoder = HypergraphDecoder::new(2, 0, vec![event(1, 0, 0.1)]).unwrap();
    assert!(matches!(decoder.decode(2, 10), Err(Err::NoSolution { .. })));
    assert_eq!(
        decoder.decode(1, 1),
        Err(Err::BudgetExceeded { visited_nodes: 1 })
    );
    assert_eq!(decoder.decode(1, 0), Err(Err::ZeroNodeBudget));
    // The skip branch discovers a valid candidate, but search has not proved
    // that the more likely first event is better. No incumbent may escape.
    let decoder = HypergraphDecoder::new(1, 0, vec![event(1, 0, 0.2), event(1, 0, 0.1)]).unwrap();
    assert_eq!(
        decoder.decode(1, 4),
        Err(Err::BudgetExceeded { visited_nodes: 4 })
    );
    assert_eq!(decoder.decode(1, 20).unwrap().selected_mechanisms, 1);
}

#[test]
fn validates_all_probabilities_masks_and_bounds() {
    for p in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        -0.1,
        0.0,
        1.0,
        1.1,
    ] {
        assert!(matches!(
            HypergraphDecoder::new(1, 1, vec![event(1, 1, p)]),
            Err(Err::InvalidProbability { mechanism: 0 })
        ));
    }
    assert!(matches!(
        HypergraphDecoder::new(129, 0, vec![]),
        Err(Err::InvalidBitCount)
    ));
    assert!(matches!(
        HypergraphDecoder::new(0, 129, vec![]),
        Err(Err::InvalidBitCount)
    ));
    assert!(matches!(
        HypergraphDecoder::new(1, 0, vec![event(2, 0, 0.1)]),
        Err(Err::DetectorMaskOutOfRange { .. })
    ));
    assert!(matches!(
        HypergraphDecoder::new(1, 0, vec![event(1, 1, 0.1)]),
        Err(Err::LogicalMaskOutOfRange { .. })
    ));
    assert!(matches!(
        HypergraphDecoder::new(1, 0, vec![event(1, 0, 0.1); 65]),
        Err(Err::TooManyMechanisms)
    ));
    let decoder = HypergraphDecoder::new(1, 0, vec![]).unwrap();
    assert_eq!(decoder.decode(2, 10), Err(Err::SyndromeOutOfRange));
}

#[test]
fn empty_model_and_full_width_masks_are_safe() {
    let empty = HypergraphDecoder::new(0, 0, vec![]).unwrap();
    assert_eq!(empty.decode(0, 1).unwrap().selected_mechanisms, 0);
    let decoder = HypergraphDecoder::new(128, 128, vec![event(1 << 127, 1 << 127, 0.5)]).unwrap();
    assert_eq!(decoder.decode(1 << 127, 10).unwrap().logical_mask, 1 << 127);
    let mut events = vec![event(0, 0, 0.01); 64];
    events[63] = event(1, 1, 0.9);
    assert_eq!(
        HypergraphDecoder::new(1, 1, events)
            .unwrap()
            .decode(1, 1)
            .unwrap()
            .selected_mechanisms,
        1 << 63
    );
    let tiny = HypergraphDecoder::new(1, 0, vec![event(1, 0, f64::from_bits(1))]).unwrap();
    assert!(tiny.decode(1, 10).unwrap().log_odds_cost.is_finite());
}

#[test]
fn most_likely_error_is_not_most_likely_logical_coset() {
    let events = vec![event(1, 1, 0.3), event(1, 0, 0.2), event(1, 0, 0.2)];
    let result = HypergraphDecoder::new(1, 1, events.clone())
        .unwrap()
        .decode(1, 100)
        .unwrap();
    assert_eq!(result.selected_mechanisms, 1);
    assert_eq!(result.logical_mask, 1);
    // Logical 0 has greater total probability (.224 versus .204) even though
    // the single most likely configuration belongs to logical 1 (.192).
    let mut cosets = [0.0; 2];
    for mask in 0..8 {
        let mut detector = 0;
        let mut logical = 0;
        let mut p = 1.0;
        for (i, e) in events.iter().enumerate() {
            if mask & (1 << i) != 0 {
                detector ^= e.detectors;
                logical ^= e.logicals;
                p *= e.probability;
            } else {
                p *= 1.0 - e.probability;
            }
        }
        if detector == 1 {
            cosets[logical as usize] += p;
        }
    }
    assert!((cosets[0] - 0.224).abs() < 1e-12);
    assert!((cosets[1] - 0.204).abs() < 1e-12);
}
