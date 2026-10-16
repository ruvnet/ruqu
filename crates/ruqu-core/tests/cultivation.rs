use ruqu_core::{cultivation::*, types::Complex};

#[test]
fn figure18_and_figure7_match_independent_css_codewords() {
    let state = surface_injected_state();
    let m = surface_metrics(&state).unwrap();
    assert!((m[0] - 1.0).abs() < 1e-12, "{m:?}");
    // Independently apply each tensor-Pauli to all amplitudes.
    for mask in SURFACE_X_CHECKS {
        let expectation: f64 = (0..8192)
            .map(|i| (state[i].conj() * state[i ^ mask as usize]).re)
            .sum();
        assert!((expectation - 1.0).abs() < 1e-12);
    }
    for mask in SURFACE_Z_CHECKS {
        let expectation: f64 = (0..8192)
            .map(|i| {
                state[i].norm_sq()
                    * if (i & mask as usize).count_ones() % 2 == 0 {
                        1.0
                    } else {
                        -1.0
                    }
            })
            .sum();
        assert!((expectation - 1.0).abs() < 1e-12);
    }
}
#[test]
fn every_single_data_pauli_leaves_the_codespace() {
    let state = surface_injected_state();
    for q in 0..13 {
        for p in 1..4 {
            let mut faulty = vec![Complex::ZERO; 8192];
            for i in 0..8192 {
                let bit = i & (1 << q) != 0;
                let (dest, phase) = match p {
                    1 => (i ^ (1 << q), Complex::ONE),
                    2 => (i ^ (1 << q), if bit { -Complex::I } else { Complex::I }),
                    _ => (i, Complex::new(if bit { -1.0 } else { 1.0 }, 0.0)),
                };
                faulty[dest] = state[i] * phase;
            }
            let metrics = surface_metrics(&faulty).unwrap();
            assert!(metrics[1] < 1e-12, "q={q} p={p} {metrics:?}");
        }
    }
}
#[test]
fn full_published_f3_noiseless_schedule_accepts() {
    let r = run_surface_cultivation(&CultivationConfig {
        attempts: 2,
        ..Default::default()
    })
    .unwrap();
    assert_eq!((r.accepted, r.rejected), (2, 0));
    assert!((r.mean_accepted_fidelity.unwrap() - 1.0).abs() < 1e-11);
    assert_eq!(r.peak_state_bytes, 1048576);
    assert_eq!(r.measurements, 60); // 2*(12 initial+6 cat+12 final)
    assert!(r.output_error_wilson_95.unwrap()[1] > 0.5); // zero failures is not proof of zero error
}
#[test]
fn noise_is_seeded_and_rejected_work_is_charged() {
    let config = CultivationConfig {
        attempts: 4,
        gate_error: 0.02,
        preparation_error: 0.02,
        measurement_error: 0.02,
        ..Default::default()
    };
    let a = run_surface_cultivation(&config).unwrap();
    let b = run_surface_cultivation(&config).unwrap();
    assert_eq!(a, b);
    assert_eq!(a.accepted + a.rejected, config.attempts);
    assert_eq!(a.rejected_by_stage.iter().sum::<u32>(), a.rejected);
    assert!(a.rejected > 0);
    assert!(a.sampled_fault_locations > 0);
    assert!(a.executed_qubit_operations > 0);
}
#[test]
fn invalid_and_unbounded_requests_fail_before_allocation() {
    for p in [f64::NAN, f64::INFINITY, -0.1, 1.01] {
        assert!(run_surface_cultivation(&CultivationConfig {
            gate_error: p,
            ..Default::default()
        })
        .is_err());
    }
    assert!(run_surface_cultivation(&CultivationConfig {
        verification_rounds: 1,
        ..Default::default()
    })
    .is_err());
    assert!(run_surface_cultivation(&CultivationConfig {
        attempts: 10001,
        ..Default::default()
    })
    .is_err());
}

#[test]
#[ignore = "exhaustive 2485-trajectory audit; run explicitly in release mode"]
fn every_single_circuit_fault_is_benign_or_detected_by_ideal_final_detection() {
    let r = audit_surface_single_faults();
    assert_eq!(r.circuit_locations, 253);
    assert_eq!(r.fault_mechanisms, 2485);
    assert_eq!(r.nonzero_accepted_logical_error, 0, "{r:?}");
    assert!(r.max_acceptance_weighted_logical_error < 1e-10, "{r:?}");
    assert!(
        r.nonzero_accepted_raw_infidelity > 0,
        "correctable residual errors must remain visible"
    );
}
