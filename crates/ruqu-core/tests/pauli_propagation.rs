use ruqu_core::pauli_propagation::*;
use ruqu_core::{gate::Gate, state::QuantumState, types::Complex};
fn p(s: &str) -> Pauli {
    Pauli::parse(s).unwrap()
}
fn apply_pauli(a: &[Complex], p: &Pauli) -> Vec<Complex> {
    let mut out = vec![Complex::ZERO; a.len()];
    for (i, &value) in a.iter().enumerate() {
        let mut j = i;
        let mut phase = Complex::ONE;
        for (q, &v) in p.0.iter().enumerate() {
            let bit = (i >> q) & 1;
            match v {
                1 => j ^= 1 << q,
                2 => {
                    if bit == 1 {
                        phase = -phase;
                    }
                }
                3 => {
                    j ^= 1 << q;
                    phase = phase * if bit == 0 { Complex::I } else { -Complex::I };
                }
                _ => {}
            }
        }
        out[j] += value * phase;
    }
    out
}
fn reference(inputs: &[InputState], ops: &[Operation], obs: &Pauli) -> (f64, f64) {
    let n = inputs.len() as u32;
    let mut initial = QuantumState::new_with_seed(n, 42).unwrap();
    for (q, s) in inputs.iter().enumerate() {
        match s {
            InputState::Zero => {}
            InputState::Plus => {
                initial.apply_gate(&Gate::H(q as u32)).unwrap();
            }
            InputState::T => {
                initial.apply_gate(&Gate::H(q as u32)).unwrap();
                initial.apply_gate(&Gate::T(q as u32)).unwrap();
            }
        }
    }
    let mut states = vec![(1.0, initial.state_vector().to_vec())];
    for op in ops {
        let mut next = vec![];
        for (weight, mut a) in states {
            match op {
                Operation::Gate(g) => {
                    let mut s = QuantumState::from_amplitudes(a, n).unwrap();
                    s.apply_gate(g).unwrap();
                    next.push((weight, s.state_vector().to_vec()));
                }
                Operation::PauliFault { pauli, probability } => {
                    next.push((weight * probability, apply_pauli(&a, pauli)));
                    next.push((weight * (1.0 - probability), a));
                }
                Operation::Postselect { pauli, positive } => {
                    let b = apply_pauli(&a, pauli);
                    for (x, y) in a.iter_mut().zip(b) {
                        *x = (*x + y * if *positive { 1.0 } else { -1.0 }) * 0.5;
                    }
                    next.push((weight, a));
                }
            }
        }
        states = next;
    }
    let mut norm = 0.0;
    let mut expected = 0.0;
    for (w, a) in states {
        norm += w * a.iter().map(|z| z.norm_sq()).sum::<f64>();
        expected += w * a
            .iter()
            .zip(apply_pauli(&a, obs))
            .map(|(x, y)| (x.conj() * y).re)
            .sum::<f64>();
    }
    (norm, expected)
}
#[test]
fn dense_reference_clifford_t_noise_postselection() {
    for seed in 0..64 {
        let inputs = [InputState::T, InputState::Zero, InputState::Plus];
        let mut ops = vec![];
        let mut rng = seed + 11u64;
        for _ in 0..14 {
            rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
            let q = (rng % 3) as u32;
            let g = match (rng >> 8) % 10 {
                0 => Gate::H(q),
                1 => Gate::S(q),
                2 => Gate::Sdg(q),
                3 => Gate::T(q),
                4 => Gate::Tdg(q),
                5 => Gate::CNOT(q, (q + 1) % 3),
                6 => Gate::CZ(q, (q + 2) % 3),
                7 => Gate::X(q),
                8 => Gate::Y(q),
                _ => Gate::SWAP(q, (q + 1) % 3),
            };
            ops.push(Operation::Gate(g));
        }
        ops.push(Operation::PauliFault {
            pauli: p("XYI"),
            probability: 0.13,
        });
        ops.push(Operation::Postselect {
            pauli: p("ZZI"),
            positive: seed % 2 == 0,
        });
        ops.push(Operation::Gate(Gate::T(1)));
        ops.push(Operation::PauliFault {
            pauli: p("IZX"),
            probability: 0.07,
        });
        ops.push(Operation::Postselect {
            pauli: p("IXY"),
            positive: true,
        });
        for obs in ["III", "XYZ", "ZZI", "YXY", "XIX"] {
            let obs = p(obs);
            let (norm, weighted) = reference(&inputs, &ops, &obs);
            let r = evaluate(&inputs, &ops, &obs, Budget::default()).unwrap();
            assert!(
                (r.acceptance_probability - norm).abs() < 1e-11,
                "seed {seed}"
            );
            assert!(
                (r.weighted_expectation - weighted).abs() < 1e-11,
                "seed {seed}: {weighted} != {}",
                r.weighted_expectation
            );
        }
    }
}
#[test]
fn encoded_t_repetition_detects_single_bit_faults() {
    let probability = 0.02;
    let mut ops = vec![
        Operation::Gate(Gate::CNOT(0, 1)),
        Operation::Gate(Gate::CNOT(0, 2)),
    ];
    for s in ["XII", "IXI", "IIX"] {
        ops.push(Operation::PauliFault {
            pauli: p(s),
            probability,
        });
    }
    for s in ["ZZI", "IZZ"] {
        ops.push(Operation::Postselect {
            pauli: p(s),
            positive: true,
        });
    }
    let r = evaluate(
        &[InputState::T, InputState::Zero, InputState::Zero],
        &ops,
        &p("YXX"),
        Budget::default(),
    )
    .unwrap();
    let accepted = (1.0 - probability).powi(3) + probability.powi(3);
    assert!((r.acceptance_probability - accepted).abs() < 1e-12);
    let logical_y = std::f64::consts::FRAC_1_SQRT_2 * (1.0 - 2.0 * probability.powi(3) / accepted);
    assert!((r.conditional_expectation.unwrap() - logical_y).abs() < 1e-12);
}
#[test]
fn large_register_needs_no_statevector() {
    let n = 128;
    let mut inputs = vec![InputState::Zero; n];
    inputs[0] = InputState::T;
    let ops = (1..n)
        .map(|q| Operation::Gate(Gate::CNOT(0, q as u32)))
        .collect::<Vec<_>>();
    let mut obs = Pauli(vec![1; n]);
    obs.0[0] = 3;
    let r = evaluate(&inputs, &ops, &obs, Budget::default()).unwrap();
    assert_eq!(r.peak_terms, 1);
    assert!((r.conditional_expectation.unwrap() - std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-12);
}
#[test]
fn impossible_selection_is_not_success() {
    let r = evaluate(
        &[InputState::Zero],
        &[Operation::Postselect {
            pauli: p("Z"),
            positive: false,
        }],
        &p("X"),
        Budget::default(),
    )
    .unwrap();
    assert_eq!(r.acceptance_probability, 0.0);
    assert!(r.conditional_expectation.is_none());
}
#[test]
fn invalid_inputs_and_budgets_fail_closed() {
    for probability in [f64::NAN, f64::INFINITY, -0.1, 1.1] {
        assert!(evaluate(
            &[InputState::T],
            &[Operation::PauliFault {
                pauli: p("X"),
                probability
            }],
            &p("Y"),
            Budget::default()
        )
        .is_err());
    }
    assert!(evaluate(
        &[InputState::T],
        &[Operation::Gate(Gate::T(0))],
        &p("X"),
        Budget {
            max_terms: 1,
            max_work: 100
        }
    )
    .is_err());
    assert!(evaluate(
        &[InputState::T],
        &[Operation::Gate(Gate::H(0))],
        &p("X"),
        Budget {
            max_terms: 4,
            max_work: 1
        }
    )
    .is_err());
    assert!(evaluate(
        &[InputState::T],
        &[Operation::Gate(Gate::CNOT(0, 0))],
        &p("X"),
        Budget::default()
    )
    .is_err());
    assert!(evaluate(
        &[InputState::T],
        &[Operation::Gate(Gate::Rx(0, 0.1))],
        &p("X"),
        Budget::default()
    )
    .is_err());
    assert!(evaluate(&[InputState::T], &[], &Pauli(vec![8]), Budget::default()).is_err());
}

#[test]
fn positive_branches_never_underflow_to_impossible_selection() {
    let ops = (0..2200)
        .map(|i| Operation::Postselect {
            pauli: p(if i % 2 == 0 { "X" } else { "Z" }),
            positive: true,
        })
        .collect::<Vec<_>>();
    assert!(evaluate(&[InputState::Zero], &ops, &p("I"), Budget::default()).is_err());
    assert!(evaluate(
        &vec![InputState::T; 4096],
        &[],
        &Pauli(vec![1; 4096]),
        Budget {
            max_terms: 1,
            max_work: 100_000
        }
    )
    .is_err());
}
#[test]
fn validation_and_final_reduction_are_budgeted() {
    assert!(evaluate(
        &vec![InputState::Zero; 4096],
        &[],
        &Pauli::identity(4096),
        Budget {
            max_terms: 1,
            max_work: 1
        }
    )
    .is_err());
    assert!(evaluate(
        &[InputState::Zero],
        &[],
        &p("I"),
        Budget {
            max_terms: 1,
            max_work: 4
        }
    )
    .is_err());
    let result = evaluate(
        &[InputState::Zero],
        &[],
        &p("I"),
        Budget {
            max_terms: 1,
            max_work: 5,
        },
    )
    .unwrap();
    assert_eq!(result.work, 5);
}
