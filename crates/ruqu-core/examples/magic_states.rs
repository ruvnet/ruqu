//! Run: cargo run -p ruqu-core --example magic_states
//! Costs are illustrative inputs, not measured hardware resource estimates.
use ruqu_core::magic_states::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args == ["--benchmark"] {
        benchmark();
        return Ok(());
    }
    let mut input_error = 0.01_f64;
    let mut shots = 100_000u64;
    let mut seed = 42u64;
    let mut rounds = 2usize;
    let mut seen = std::collections::HashSet::new();
    let mut pairs = args.chunks_exact(2);
    for pair in &mut pairs {
        let flag = pair[0].as_str();
        let value = pair[1].as_str();
        if !seen.insert(flag) {
            return Err(format!("duplicate argument: {flag}").into());
        }
        match flag {
            "--input-error" => {
                input_error = value.parse()?;
            }
            "--shots" | "--seed" | "--rounds" => {
                if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
                    return Err(format!("{flag} requires an unsigned decimal integer").into());
                }
                match flag {
                    "--shots" => shots = value.parse()?,
                    "--seed" => seed = value.parse()?,
                    _ => rounds = value.parse()?,
                }
            }
            _ => return Err(format!("unknown argument: {flag}").into()),
        }
    }
    if !pairs.remainder().is_empty() {
        return Err("missing argument value".into());
    }
    if !(1..=5_000_000).contains(&shots) {
        return Err("shots must be in 1..=5000000".into());
    }
    if !(1..=16).contains(&rounds) {
        return Err("rounds must be in 1..=16".into());
    }
    let exact = distill_15_to_1(input_error)?;
    let sampled = sample_15_to_1(input_error, shots, seed)?;
    let estimate = distillation_factory(
        input_error,
        rounds,
        AttemptCost {
            qubits: 15.0,
            rounds: 10.0,
        },
        AttemptCost {
            qubits: 1.0,
            rounds: 1.0,
        },
    )?;
    let transferred = apply_transfer(
        estimate,
        0.9,
        1e-6,
        AttemptCost {
            qubits: 20.0,
            rounds: 5.0,
        },
    )?;
    println!("{{\"model\":\"ideal_15_to_1_independent_Z\",\"physical_cultivation_reproduction\":false,\"costs\":\"illustrative\",\"input_error\":{},\"rounds\":{},\"acceptance\":{},\"conditional_output_error\":{},\"attempts\":{},\"accepted\":{},\"rejected\":{},\"logical_errors\":{},\"seed\":{},\"factory_transfer\":{{\"expected_raw_states\":{},\"expected_stage_attempts\":{},\"expected_qubit_rounds\":{},\"output_error_upper_bound\":{}}}}}",
        input_error, rounds, exact.acceptance_probability, exact.conditional_output_error,
        sampled.attempts, sampled.accepted, sampled.rejected, sampled.logical_errors, sampled.seed,
        transferred.expected_raw_states, transferred.expected_stage_attempts,
        transferred.expected_qubit_rounds, transferred.output_error_upper_bound);
    Ok(())
}

// Reproducible cold-setup microbenchmark. Run a release build and compare on
// the same machine; results are measured elapsed time, not portable guarantees.
fn benchmark() {
    use std::{hint::black_box, time::Instant};
    fn exhaustive() -> ([u32; 16], [u32; 16]) {
        let mut good = [0u32; 16];
        let mut bad = [0u32; 16];
        for mask in 0u16..32768 {
            let outcome = classify_faults(mask).unwrap();
            if outcome.accepted() {
                let target = if outcome.logical_error {
                    &mut bad
                } else {
                    &mut good
                };
                target[mask.count_ones() as usize] += 1;
            }
        }
        (good, bad)
    }
    let iterations = 500u32;
    let start = Instant::now();
    for _ in 0..iterations {
        black_box(compute_weight_enumerators());
    }
    let dp_ns = start.elapsed().as_nanos();
    let start = Instant::now();
    for _ in 0..iterations {
        black_box(exhaustive());
    }
    let enumeration_ns = start.elapsed().as_nanos();
    let start = Instant::now();
    for _ in 0..iterations {
        black_box(weight_enumerators());
    }
    let cached_ns = start.elapsed().as_nanos();
    println!("{{\"benchmark\":\"magic_state_weight_spectrum\",\"iterations\":{},\"cold_dp_total_ns\":{},\"enumeration_total_ns\":{},\"cached_total_ns\":{},\"matching_spectra\":{}}}",
        iterations, dp_ns, enumeration_ns, cached_ns,
        compute_weight_enumerators() == exhaustive());
}
