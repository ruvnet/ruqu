use ruqu_core::{gate::Gate, pauli_propagation::*};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let n = 128;
    let mut inputs = vec![InputState::Zero; n];
    inputs[0] = InputState::T;
    let operations = (1..n)
        .map(|q| Operation::Gate(Gate::CNOT(0, q as u32)))
        .collect::<Vec<_>>();
    let mut observable = Pauli(vec![1; n]);
    observable.0[0] = 3;
    let start = std::time::Instant::now();
    let result = evaluate(&inputs, &operations, &observable, Budget::default())?;
    println!("{{\"model\":\"encoded_T_Clifford_observable\",\"qubits\":{n},\"acceptance\":{},\"expectation\":{},\"peak_terms\":{},\"work\":{},\"elapsed_us\":{},\"hardware_run\":false}}",result.acceptance_probability,result.conditional_expectation.unwrap(),result.peak_terms,result.work,start.elapsed().as_micros());
    Ok(())
}
