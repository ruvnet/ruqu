use ruqu_core::magic_states::*;

#[test]
fn reed_muller_weight_spectrum_and_detection_distance() {
    let (good, bad) = weight_enumerators();
    assert_eq!(good.iter().sum::<u32>(), 1024);
    assert_eq!(bad.iter().sum::<u32>(), 1024);
    assert_eq!(bad[1], 0);
    assert_eq!(bad[2], 0);
    assert_eq!(bad[3], 35);
    assert_eq!(good[0], 1);
    for i in 0..15 {
        assert!(!classify_faults(1 << i).unwrap().accepted());
    }
    assert!(classify_faults(0x8000).is_err());
}

#[test]
fn independent_macwilliams_closed_form_reference() {
    for p in [0.01_f64, 0.05, 0.1, 0.141, 0.25, 0.5, 0.9, 1.0] {
        let t = 1.0 - 2.0 * p;
        // Characteristic-function reference from BK 2005, equations 35-36.
        // Independent of the implementation's error-mask enumeration.
        let acceptance = (1.0 + 15.0 * t.powi(8)) / 16.0;
        let output = (1.0 - 15.0 * t.powi(7) + 15.0 * t.powi(8) - t.powi(15))
            / (2.0 * (1.0 + 15.0 * t.powi(8)));
        let exact = distill_15_to_1(p).unwrap();
        assert!((exact.acceptance_probability - acceptance).abs() < 1e-14);
        assert!((exact.conditional_output_error - output).abs() < 1e-14);
        assert!((exact.rejected_probability + exact.acceptance_probability - 1.0).abs() < 1e-15);
    }
}

#[test]
fn tiny_error_retains_cubic_signal_without_subtractive_cancellation() {
    let p = 1e-10;
    let exact = distill_15_to_1(p).unwrap();
    assert!((exact.conditional_output_error / (35.0 * p.powi(3)) - 1.0).abs() < 2e-9);
    assert_eq!(distill_15_to_1(0.0).unwrap().conditional_output_error, 0.0);
    assert_eq!(distill_15_to_1(1.0).unwrap().conditional_output_error, 1.0);
}

#[test]
fn dense_error_register_projector_reference() {
    // Independent dense amplitudes for a coherent error register, then project
    // with the four parity-check operators and measure logical parity. This is
    // a reference for the fault model, not a physical distillation circuit.
    let p = 0.073_f64;
    let mut amplitudes = vec![0.0; 1 << 15];
    amplitudes[0] = 1.0;
    for qubit in 0..15 {
        for basis in 0..(1 << qubit) {
            amplitudes[basis | (1 << qubit)] = amplitudes[basis] * p.sqrt();
            amplitudes[basis] *= (1.0 - p).sqrt();
        }
    }
    let row_masks = [0x5555usize, 0x6666, 0x7878, 0x7f80];
    let mut accepted = 0.0;
    let mut bad = 0.0;
    for (basis, amplitude) in amplitudes.iter().enumerate() {
        if row_masks
            .iter()
            .all(|row| (basis & row).count_ones() % 2 == 0)
        {
            accepted += amplitude * amplitude;
            if basis.count_ones() % 2 == 1 {
                bad += amplitude * amplitude;
            }
        }
    }
    let result = distill_15_to_1(p).unwrap();
    assert!((accepted - result.acceptance_probability).abs() < 1e-13);
    assert!((bad / accepted - result.conditional_output_error).abs() < 1e-13);
}

#[test]
fn seeded_sampling_accounts_for_rejections_and_matches_exact() {
    let mc = sample_15_to_1(0.1, 200_000, 42).unwrap();
    assert_eq!(mc, sample_15_to_1(0.1, 200_000, 42).unwrap());
    assert_eq!(mc.attempts, mc.accepted + mc.rejected);
    assert!(mc.rejected > 0);
    assert!(mc.logical_errors <= mc.accepted);
    let exact = distill_15_to_1(0.1).unwrap();
    let (acceptance, output) = mc.confidence_bounds(1e-6).unwrap();
    assert!(
        acceptance.lower <= exact.acceptance_probability
            && exact.acceptance_probability <= acceptance.upper
    );
    assert!(
        output.lower <= exact.conditional_output_error
            && exact.conditional_output_error <= output.upper
    );
    let empty = MonteCarloCounts {
        attempts: 10,
        accepted: 0,
        rejected: 10,
        logical_errors: 0,
        seed: 0,
    };
    assert_eq!(empty.output_error_estimate(), None);
    let (_, bounds) = empty.confidence_bounds(0.05).unwrap();
    assert_eq!((bounds.lower, bounds.upper), (0.0, 1.0));
    assert!(empty
        .confidence_bounds(f64::from_bits(1))
        .unwrap()
        .0
        .upper
        .is_finite());
}

#[test]
fn retries_stages_and_transfer_charge_all_consumed_resources() {
    let stage = FactoryStage {
        input_states: 15,
        acceptance_probability: 0.5,
        conditional_output_error_upper_bound: 1e-4,
        attempt_cost: AttemptCost {
            qubits: 10.0,
            rounds: 2.0,
        },
    };
    let estimate = estimate_factory(
        &[stage, stage],
        AttemptCost {
            qubits: 1.0,
            rounds: 1.0,
        },
    )
    .unwrap();
    assert_eq!(estimate.expected_raw_states, 900.0);
    assert_eq!(estimate.expected_stage_attempts, 62.0);
    assert_eq!(estimate.expected_qubit_rounds, 2140.0);
    let transferred = apply_transfer(
        estimate,
        0.5,
        1e-5,
        AttemptCost {
            qubits: 2.0,
            rounds: 5.0,
        },
    )
    .unwrap();
    assert_eq!(transferred.expected_raw_states, 1800.0);
    assert_eq!(transferred.expected_stage_attempts, 126.0);
    assert_eq!(transferred.expected_qubit_rounds, 4300.0);
    assert!((transferred.output_error_upper_bound - 0.00021).abs() < 1e-15);
    let two = distillation_factory(
        0.01,
        2,
        stage.attempt_cost,
        AttemptCost {
            qubits: 0.0,
            rounds: 0.0,
        },
    )
    .unwrap();
    let one = distill_15_to_1(0.01).unwrap();
    let expected = distill_15_to_1(one.conditional_output_error).unwrap();
    assert_eq!(
        two.output_error_upper_bound,
        expected.conditional_output_error
    );
    assert!(two.expected_raw_states > 225.0);
}

#[test]
fn invalid_inputs_zero_yield_and_overflow_are_errors() {
    for p in [f64::NAN, f64::INFINITY, -0.1, 1.1] {
        assert_eq!(
            distill_15_to_1(p).unwrap_err(),
            FactoryError::InvalidProbability
        );
    }
    assert_eq!(
        sample_15_to_1(0.1, 0, 0).unwrap_err(),
        FactoryError::EmptySamples
    );
    let zero = AttemptCost {
        qubits: 0.0,
        rounds: 0.0,
    };
    let mut stage = FactoryStage {
        input_states: 1,
        acceptance_probability: 0.0,
        conditional_output_error_upper_bound: 0.0,
        attempt_cost: zero,
    };
    assert_eq!(
        estimate_factory(&[stage], zero).unwrap_err(),
        FactoryError::ZeroYield
    );
    stage.acceptance_probability = f64::from_bits(1);
    assert_eq!(
        estimate_factory(&[stage], zero).unwrap_err(),
        FactoryError::Overflow
    );
    assert_eq!(
        estimate_factory(&[], zero).unwrap_err(),
        FactoryError::InvalidResources
    );
    assert_eq!(
        distillation_factory(0.1, 0, zero, zero).unwrap_err(),
        FactoryError::InvalidResources
    );
    assert_eq!(
        distillation_factory(
            0.1,
            1,
            AttemptCost {
                qubits: f64::MAX,
                rounds: 2.0
            },
            zero
        )
        .unwrap_err(),
        FactoryError::Overflow
    );
}

#[test]
fn dynamic_programming_matches_exhaustive_parity_projectors() {
    let mut good = [0u32; 16];
    let mut bad = [0u32; 16];
    for mask in 0u16..32768 {
        // Independent explicit check-row reference, not the DP transition.
        if [0x5555u16, 0x6666, 0x7878, 0x7f80]
            .iter()
            .all(|row| (mask & row).count_ones() % 2 == 0)
        {
            let target = if mask.count_ones() % 2 == 0 {
                &mut good
            } else {
                &mut bad
            };
            target[mask.count_ones() as usize] += 1;
        }
    }
    assert_eq!(compute_weight_enumerators(), (good, bad));
    assert_eq!(weight_enumerators(), (good, bad));
}

#[test]
fn transfer_postselection_can_concentrate_existing_errors() {
    let estimate = FactoryEstimate {
        expected_raw_states: 1.0,
        expected_stage_attempts: 1.0,
        expected_qubit_rounds: 1.0,
        output_error_upper_bound: 0.1,
    };
    // Accept exactly the faulty tenth of the population: conditional error is
    // one even though the transfer introduces no additional faults.
    let selected = apply_transfer(
        estimate,
        0.1,
        0.0,
        AttemptCost {
            qubits: 0.0,
            rounds: 0.0,
        },
    )
    .unwrap();
    assert_eq!(selected.output_error_upper_bound, 1.0);
    assert_eq!(selected.expected_raw_states, 10.0);
}

#[test]
fn positive_input_never_reports_perfect_output_after_underflow() {
    assert_eq!(
        distill_15_to_1(1e-200).unwrap_err(),
        FactoryError::NumericalUnderflow
    );
    let zero = AttemptCost {
        qubits: 0.0,
        rounds: 0.0,
    };
    assert_eq!(
        distillation_factory(1e-50, 3, zero, zero).unwrap_err(),
        FactoryError::NumericalUnderflow
    );
    assert_eq!(distill_15_to_1(0.0).unwrap().conditional_output_error, 0.0);
}
