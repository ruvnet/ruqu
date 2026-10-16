//! Run with cargo run -p ruqu-core --release --example cultivation.
use ruqu_core::cultivation::{run_surface_cultivation, CultivationConfig};
fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("--ideal") {
        let (p, m) = ruqu_core::cultivation::surface_ideal_branch();
        println!("[{}, {:?}]", p, m);
        return;
    }
    if args.get(1).map(String::as_str) == Some("--trace") {
        let trace = ruqu_core::cultivation::surface_circuit_trace();
        println!("[");
        for (i, (name, qs)) in trace.iter().enumerate() {
            println!(
                "[{:?},{:?}]{}",
                name,
                qs,
                if i + 1 < trace.len() { "," } else { "" }
            );
        }
        println!("]");
        return;
    }
    if args.get(1).map(String::as_str) == Some("--fault") {
        let (p, m) = ruqu_core::cultivation::surface_fault_branch(
            args[2].parse().unwrap(),
            args[3].parse().unwrap(),
        )
        .unwrap();
        println!("[{}, {:?}]", p, m);
        return;
    }
    if std::env::args().any(|arg| arg == "--audit") {
        println!(
            "{:#?}",
            ruqu_core::cultivation::audit_surface_single_faults()
        );
        return;
    }
    for p in [0.0, 0.001, 0.01] {
        let report = run_surface_cultivation(&CultivationConfig {
            attempts: 16,
            gate_error: p,
            preparation_error: p,
            measurement_error: p,
            ..Default::default()
        })
        .expect("bounded config");
        println!("{report:#?}");
    }
}
