//! # ruqu-wasm - WebAssembly Quantum Simulation
//!
//! Browser-compatible quantum circuit simulation.
//! Supports up to 25 qubits in WASM (memory limit enforcement).
//!
//! This crate provides wasm-bindgen bindings over `ruqu-core` and `ruqu-algorithms`,
//! exposing a JavaScript-friendly API for building quantum circuits, running simulations,
//! and executing quantum algorithms (Grover's search, QAOA MaxCut) directly in the browser.
//!
//! ## Usage (JavaScript)
//!
//! ```javascript
//! import { WasmQuantumCircuit, simulate, max_qubits, estimate_memory } from 'ruqu-wasm';
//!
//! // Check limits
//! console.log(`Max qubits: ${max_qubits()}`);
//! console.log(`Memory for 10 qubits: ${estimate_memory(10)} bytes`);
//!
//! // Build a Bell state circuit
//! const circuit = new WasmQuantumCircuit(2);
//! circuit.h(0);
//! circuit.cnot(0, 1);
//! circuit.measure_all();
//!
//! // Simulate
//! const result = simulate(circuit);
//! console.log(result.probabilities);
//! ```
//!
//! ## Memory Limits
//!
//! WASM operates under 32-bit address space constraints (~4GB max).
//! A quantum state vector for n qubits requires `2^n * 16` bytes
//! (complex f64 amplitudes). At 25 qubits this is ~512MB, which is
//! a practical upper bound for browser environments.

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

/// Maximum qubits allowed in WASM environment.
///
/// 25 qubits produces a state vector of 2^25 = 33,554,432 complex amplitudes,
/// requiring approximately 512MB (at 16 bytes per complex f64 pair).
/// This is near the practical limit for 32-bit WASM address space.
const WASM_MAX_QUBITS: u32 = 25;

// ═══════════════════════════════════════════════════════════════════════════
// WasmQuantumCircuit - JS-friendly circuit builder
// ═══════════════════════════════════════════════════════════════════════════

/// A JavaScript-friendly quantum circuit builder.
///
/// Wraps `ruqu_core::circuit::QuantumCircuit` with wasm-bindgen annotations.
/// All gate methods validate qubit indices against the circuit size internally
/// via the core library.
///
/// ## JavaScript Example
///
/// ```javascript
/// const qc = new WasmQuantumCircuit(3);
/// qc.h(0);           // Hadamard on qubit 0
/// qc.cnot(0, 1);     // CNOT: control=0, target=1
/// qc.rz(2, Math.PI); // Rz rotation on qubit 2
/// qc.measure_all();
///
/// console.log(`Qubits: ${qc.num_qubits}`);
/// console.log(`Gates:  ${qc.gate_count}`);
/// console.log(`Depth:  ${qc.depth}`);
/// ```
#[wasm_bindgen]
pub struct WasmQuantumCircuit {
    inner: ruqu_core::circuit::QuantumCircuit,
}

#[wasm_bindgen]
impl WasmQuantumCircuit {
    /// Create a new quantum circuit with the given number of qubits.
    ///
    /// Returns an error if `num_qubits` exceeds the WASM limit (25).
    #[wasm_bindgen(constructor)]
    pub fn new(num_qubits: u32) -> Result<WasmQuantumCircuit, JsValue> {
        if num_qubits > WASM_MAX_QUBITS {
            return Err(JsValue::from_str(&format!(
                "Qubit limit exceeded: {} requested, max {} in WASM",
                num_qubits, WASM_MAX_QUBITS
            )));
        }
        Ok(Self {
            inner: ruqu_core::circuit::QuantumCircuit::new(num_qubits),
        })
    }

    // ── Single-qubit gates ──────────────────────────────────────────────

    /// Apply Hadamard gate to the target qubit.
    pub fn h(&mut self, qubit: u32) {
        self.inner.h(qubit);
    }

    /// Apply Pauli-X (NOT) gate to the target qubit.
    pub fn x(&mut self, qubit: u32) {
        self.inner.x(qubit);
    }

    /// Apply Pauli-Y gate to the target qubit.
    pub fn y(&mut self, qubit: u32) {
        self.inner.y(qubit);
    }

    /// Apply Pauli-Z gate to the target qubit.
    pub fn z(&mut self, qubit: u32) {
        self.inner.z(qubit);
    }

    /// Apply S (phase) gate to the target qubit.
    pub fn s(&mut self, qubit: u32) {
        self.inner.s(qubit);
    }

    /// Apply T gate to the target qubit.
    pub fn t(&mut self, qubit: u32) {
        self.inner.t(qubit);
    }

    /// Apply Rx rotation gate with the given angle (radians).
    ///
    /// Throws if `angle` is NaN or infinite.
    pub fn rx(&mut self, qubit: u32, angle: f64) -> Result<(), JsValue> {
        check_angle("rx", angle).map_err(|e| JsValue::from_str(&e))?;
        self.inner.rx(qubit, angle);
        Ok(())
    }

    /// Apply Ry rotation gate with the given angle (radians).
    ///
    /// Throws if `angle` is NaN or infinite.
    pub fn ry(&mut self, qubit: u32, angle: f64) -> Result<(), JsValue> {
        check_angle("ry", angle).map_err(|e| JsValue::from_str(&e))?;
        self.inner.ry(qubit, angle);
        Ok(())
    }

    /// Apply Rz rotation gate with the given angle (radians).
    ///
    /// Throws if `angle` is NaN or infinite.
    pub fn rz(&mut self, qubit: u32, angle: f64) -> Result<(), JsValue> {
        check_angle("rz", angle).map_err(|e| JsValue::from_str(&e))?;
        self.inner.rz(qubit, angle);
        Ok(())
    }

    // ── Two-qubit gates ─────────────────────────────────────────────────

    /// Apply CNOT (controlled-X) gate.
    pub fn cnot(&mut self, control: u32, target: u32) {
        self.inner.cnot(control, target);
    }

    /// Apply controlled-Z gate.
    pub fn cz(&mut self, q1: u32, q2: u32) {
        self.inner.cz(q1, q2);
    }

    /// Apply SWAP gate.
    pub fn swap(&mut self, q1: u32, q2: u32) {
        self.inner.swap(q1, q2);
    }

    /// Apply Rzz (ZZ-rotation) gate with the given angle (radians).
    ///
    /// Throws if `angle` is NaN or infinite.
    pub fn rzz(&mut self, q1: u32, q2: u32, angle: f64) -> Result<(), JsValue> {
        check_angle("rzz", angle).map_err(|e| JsValue::from_str(&e))?;
        self.inner.rzz(q1, q2, angle);
        Ok(())
    }

    // ── Measurement and control ─────────────────────────────────────────

    /// Add a measurement operation on a single qubit.
    pub fn measure(&mut self, qubit: u32) {
        self.inner.measure(qubit);
    }

    /// Add measurement operations on all qubits.
    pub fn measure_all(&mut self) {
        self.inner.measure_all();
    }

    /// Reset a qubit to the |0> state.
    pub fn reset(&mut self, qubit: u32) {
        self.inner.reset(qubit);
    }

    /// Insert a barrier (prevents gate reordering across this point).
    pub fn barrier(&mut self) {
        self.inner.barrier();
    }

    // ── Circuit properties ──────────────────────────────────────────────

    /// The number of qubits in this circuit.
    #[wasm_bindgen(getter)]
    pub fn num_qubits(&self) -> u32 {
        self.inner.num_qubits()
    }

    /// The total number of gates applied so far.
    #[wasm_bindgen(getter)]
    pub fn gate_count(&self) -> usize {
        self.inner.gate_count()
    }

    /// The circuit depth (longest path through the gate DAG).
    #[wasm_bindgen(getter)]
    pub fn depth(&self) -> u32 {
        self.inner.depth()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Simulation result types (serialized to JS via serde-wasm-bindgen)
// ═══════════════════════════════════════════════════════════════════════════

/// Simulation result returned as a plain JS object.
///
/// Contains the probability distribution, any measurement outcomes,
/// and execution metadata.
#[derive(Serialize, Deserialize)]
pub struct WasmSimResult {
    /// Probability of each computational basis state (length = 2^n).
    pub probabilities: Vec<f64>,
    /// Measurement outcomes for qubits that were explicitly measured.
    pub measurements: Vec<WasmMeasurement>,
    /// Number of qubits in the simulated circuit.
    pub num_qubits: u32,
    /// Total gate count of the simulated circuit.
    pub gate_count: usize,
    /// Wall-clock execution time in milliseconds.
    pub execution_time_ms: f64,
}

/// A single qubit measurement outcome.
#[derive(Serialize, Deserialize)]
pub struct WasmMeasurement {
    /// Which qubit was measured.
    pub qubit: u32,
    /// The measured classical bit (true = |1>, false = |0>).
    pub result: bool,
    /// The probability of this outcome (before collapse).
    pub probability: f64,
}

// ═══════════════════════════════════════════════════════════════════════════
// Top-level simulation function
// ═══════════════════════════════════════════════════════════════════════════

/// Run a quantum circuit simulation and return the results as a JS object.
///
/// `seed` is optional. Omit it (or pass `null`/`undefined`) and any
/// `measure`/`measure_all` operation collapses with OS entropy. Pass an
/// integer in `0..=Number.MAX_SAFE_INTEGER` and the collapse is
/// reproducible. Any other seed value throws.
///
/// The returned object has the shape:
/// ```typescript
/// {
///   probabilities: number[],   // length = 2^num_qubits
///   measurements: Array<{ qubit: number, result: boolean, probability: number }>,
///   num_qubits: number,
///   gate_count: number,
///   execution_time_ms: number,
/// }
/// ```
#[wasm_bindgen]
pub fn simulate(circuit: &WasmQuantumCircuit, seed: Option<f64>) -> Result<JsValue, JsValue> {
    let seed = match seed {
        Some(value) => Some(check_seed(value).map_err(|e| JsValue::from_str(&e))?),
        None => None,
    };
    let config = ruqu_core::simulator::SimConfig {
        seed,
        ..Default::default()
    };
    let result = ruqu_core::simulator::Simulator::run_with_config(&circuit.inner, &config)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    let wasm_result = WasmSimResult {
        probabilities: result.state.probabilities(),
        measurements: result
            .measurements
            .iter()
            .map(|m| WasmMeasurement {
                qubit: m.qubit,
                result: m.result,
                probability: m.probability,
            })
            .collect(),
        num_qubits: result.metrics.num_qubits,
        gate_count: result.metrics.gate_count,
        execution_time_ms: result.metrics.execution_time_ns as f64 / 1_000_000.0,
    };

    serde_wasm_bindgen::to_value(&wasm_result).map_err(|e| JsValue::from_str(&e.to_string()))
}

// ═══════════════════════════════════════════════════════════════════════════
// Input validation (plain Rust so it is testable off wasm32)
// ═══════════════════════════════════════════════════════════════════════════

/// Reject rotation angles that would turn every amplitude into NaN.
fn check_angle(gate: &str, angle: f64) -> Result<(), String> {
    if angle.is_finite() {
        Ok(())
    } else {
        Err(format!("{gate}: angle must be a finite number, got {angle}"))
    }
}

/// Accept only seeds that a JS number represents exactly.
fn check_seed(seed: f64) -> Result<u64, String> {
    const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;
    if seed.is_finite() && seed >= 0.0 && seed.fract() == 0.0 && seed <= MAX_SAFE_INTEGER {
        Ok(seed as u64)
    } else {
        Err(format!(
            "seed must be an integer in 0..=Number.MAX_SAFE_INTEGER, null, or undefined, got {seed}"
        ))
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Utility functions
// ═══════════════════════════════════════════════════════════════════════════

/// Estimate memory usage (in bytes) for a state vector of `num_qubits` qubits.
///
/// Each qubit doubles the state vector size. The formula is `2^n * 16` bytes
/// (two f64 values per complex amplitude).
#[wasm_bindgen]
pub fn estimate_memory(num_qubits: u32) -> usize {
    ruqu_core::state::QuantumState::estimate_memory(num_qubits)
}

/// Get the maximum number of qubits supported in the WASM environment.
#[wasm_bindgen]
pub fn max_qubits() -> u32 {
    WASM_MAX_QUBITS
}

// ═══════════════════════════════════════════════════════════════════════════
// Grover's search algorithm
// ═══════════════════════════════════════════════════════════════════════════

/// Run Grover's quantum search algorithm.
///
/// Searches for one or more target states in a space of `2^num_qubits` items.
/// The optimal number of iterations is computed automatically when not specified.
///
/// ## Parameters
///
/// - `num_qubits` - Number of qubits (search space = 2^num_qubits).
/// - `target_states` - Array of target state indices to search for (as u32 values).
/// - `seed` - Optional RNG seed for reproducibility. Pass `null` or `undefined`
///            for non-deterministic execution. If provided, interpreted as a
///            floating-point number and truncated to a 64-bit unsigned integer.
///
/// ## Returns
///
/// A JS object:
/// ```typescript
/// {
///   measured_state: number,
///   target_found: boolean,
///   success_probability: number,
///   num_iterations: number,
/// }
/// ```
#[wasm_bindgen]
pub fn grover_search(
    num_qubits: u32,
    target_states: Vec<u32>,
    seed: JsValue,
) -> Result<JsValue, JsValue> {
    if num_qubits > WASM_MAX_QUBITS {
        return Err(JsValue::from_str(&format!(
            "Qubit limit exceeded: {} requested, max {} in WASM",
            num_qubits, WASM_MAX_QUBITS
        )));
    }

    // Convert seed: JsValue -> Option<u64>
    // Accept null/undefined as None, otherwise parse as f64 and truncate.
    let seed_opt: Option<u64> = if seed.is_undefined() || seed.is_null() {
        None
    } else {
        Some(
            seed.as_f64()
                .ok_or_else(|| JsValue::from_str("seed must be a number, null, or undefined"))?
                as u64,
        )
    };

    // Convert Vec<u32> -> Vec<usize> for the core API.
    let target_states_usize: Vec<usize> = target_states.into_iter().map(|s| s as usize).collect();

    let config = ruqu_algorithms::grover::GroverConfig {
        num_qubits,
        target_states: target_states_usize,
        num_iterations: None,
        seed: seed_opt,
    };

    let result = ruqu_algorithms::grover::run_grover(&config)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    #[derive(Serialize)]
    struct GroverJs {
        measured_state: usize,
        target_found: bool,
        success_probability: f64,
        num_iterations: u32,
    }

    serde_wasm_bindgen::to_value(&GroverJs {
        measured_state: result.measured_state,
        target_found: result.target_found,
        success_probability: result.success_probability,
        num_iterations: result.num_iterations,
    })
    .map_err(|e| JsValue::from_str(&e.to_string()))
}

// ═══════════════════════════════════════════════════════════════════════════
// QAOA MaxCut algorithm
// ═══════════════════════════════════════════════════════════════════════════

/// Build and simulate a QAOA (Quantum Approximate Optimization Algorithm)
/// circuit for the MaxCut problem on an undirected graph.
///
/// ## Parameters
///
/// - `num_nodes` - Number of graph nodes (one qubit per node).
/// - `edges_flat` - Flattened edge list as consecutive pairs: `[i1, j1, i2, j2, ...]`.
///   Each `(i, j)` pair defines an undirected edge with unit weight.
/// - `p` - Number of QAOA rounds (circuit depth parameter).
/// - `gammas` - Problem-unitary angles, length must equal `p`.
/// - `betas` - Mixer-unitary angles, length must equal `p`.
/// - `seed` - Optional RNG seed. Pass `null` or `undefined` for non-deterministic
///            execution.
///
/// ## Returns
///
/// A JS object:
/// ```typescript
/// {
///   probabilities: number[],   // length = 2^num_nodes
///   expected_cut: number,      // expected cut value from the output state
/// }
/// ```
#[wasm_bindgen]
pub fn qaoa_maxcut(
    num_nodes: u32,
    edges_flat: Vec<u32>,
    p: u32,
    gammas: Vec<f64>,
    betas: Vec<f64>,
    seed: JsValue,
) -> Result<JsValue, JsValue> {
    if num_nodes > WASM_MAX_QUBITS {
        return Err(JsValue::from_str(&format!(
            "Qubit limit exceeded: {} requested, max {} in WASM",
            num_nodes, WASM_MAX_QUBITS
        )));
    }

    if gammas.len() != p as usize {
        return Err(JsValue::from_str(&format!(
            "gammas length mismatch: expected {} (p), got {}",
            p,
            gammas.len()
        )));
    }
    if betas.len() != p as usize {
        return Err(JsValue::from_str(&format!(
            "betas length mismatch: expected {} (p), got {}",
            p,
            betas.len()
        )));
    }
    if edges_flat.len() % 2 != 0 {
        return Err(JsValue::from_str(
            "edges_flat must contain an even number of elements (pairs of node indices)",
        ));
    }

    // Convert seed: JsValue -> Option<u64>
    let seed_opt: Option<u64> = if seed.is_undefined() || seed.is_null() {
        None
    } else {
        Some(
            seed.as_f64()
                .ok_or_else(|| JsValue::from_str("seed must be a number, null, or undefined"))?
                as u64,
        )
    };

    // Build graph from flattened edge pairs.
    let mut graph = ruqu_algorithms::qaoa::Graph::new(num_nodes);
    for chunk in edges_flat.chunks(2) {
        if chunk.len() == 2 {
            graph.add_edge(chunk[0], chunk[1], 1.0);
        }
    }

    // Build and run the QAOA circuit.
    let circuit = ruqu_algorithms::qaoa::build_qaoa_circuit(&graph, &gammas, &betas);
    let result = ruqu_core::simulator::Simulator::run_with_config(
        &circuit,
        &ruqu_core::simulator::SimConfig {
            seed: seed_opt,
            noise: None,
            shots: None,
        },
    )
    .map_err(|e| JsValue::from_str(&e.to_string()))?;

    let probs = result.state.probabilities();

    // Compute the expected cut value: sum over edges of 0.5 * (1 - <Z_i Z_j>).
    let mut expected_cut = 0.0;
    for chunk in edges_flat.chunks(2) {
        if chunk.len() == 2 {
            let zz = result
                .state
                .expectation_value(&ruqu_core::types::PauliString {
                    ops: vec![
                        (chunk[0], ruqu_core::types::PauliOp::Z),
                        (chunk[1], ruqu_core::types::PauliOp::Z),
                    ],
                });
            expected_cut += 0.5 * (1.0 - zz);
        }
    }

    #[derive(Serialize)]
    struct QaoaJs {
        probabilities: Vec<f64>,
        expected_cut: f64,
    }

    serde_wasm_bindgen::to_value(&QaoaJs {
        probabilities: probs,
        expected_cut,
    })
    .map_err(|e| JsValue::from_str(&e.to_string()))
}

// ═══════════════════════════════════════════════════════════════════════════
// WASM initialization
// ═══════════════════════════════════════════════════════════════════════════

/// Called automatically when the WASM module is instantiated.
///
/// Sets up `console_error_panic_hook` (when the feature is enabled) so that
/// Rust panics produce readable stack traces in the browser console instead
/// of opaque "unreachable" errors.
#[wasm_bindgen(start)]
pub fn init() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

// ═══════════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════════

// Tests for the WASM bindings only run on wasm32 because wasm-bindgen
// 0.2.117 panics on `JsValue::from_str` from a non-wasm runtime.
// Native verification of the underlying logic lives in `ruqu-core`.
#[cfg(all(test, target_arch = "wasm32"))]
mod tests {
    use super::*;

    #[test]
    fn test_max_qubits_constant() {
        assert_eq!(max_qubits(), 25);
    }

    #[test]
    fn test_circuit_rejects_too_many_qubits() {
        let result = WasmQuantumCircuit::new(WASM_MAX_QUBITS + 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_circuit_accepts_max_qubits() {
        // Should not error at the boundary.
        let result = WasmQuantumCircuit::new(WASM_MAX_QUBITS);
        assert!(result.is_ok());
    }

    #[test]
    fn test_circuit_accepts_small_count() {
        let circuit = WasmQuantumCircuit::new(2).expect("2 qubits should succeed");
        assert_eq!(circuit.num_qubits(), 2);
        assert_eq!(circuit.gate_count(), 0);
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod validation_tests {
    use super::{check_angle, check_seed};

    #[test]
    fn finite_angles_pass() {
        for a in [0.0, -0.0, 1.5, -std::f64::consts::PI, 1e300] {
            assert!(check_angle("rx", a).is_ok(), "{a}");
        }
    }

    #[test]
    fn non_finite_angles_fail() {
        for a in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let err = check_angle("ry", a).unwrap_err();
            assert!(err.starts_with("ry: angle must be a finite number"), "{err}");
        }
    }

    #[test]
    fn integer_seeds_pass() {
        assert_eq!(check_seed(0.0), Ok(0));
        assert_eq!(check_seed(42.0), Ok(42));
        assert_eq!(check_seed(9_007_199_254_740_991.0), Ok(9_007_199_254_740_991));
    }

    #[test]
    fn bad_seeds_fail() {
        for s in [-1.0, 1.5, f64::NAN, f64::INFINITY, 9_007_199_254_740_992.0] {
            assert!(check_seed(s).is_err(), "{s}");
        }
    }

    #[test]
    fn seeded_measurement_is_reproducible() {
        let mut c = ruqu_core::circuit::QuantumCircuit::new(1);
        c.h(0);
        c.measure(0);
        let run = |seed| {
            let config = ruqu_core::simulator::SimConfig {
                seed: Some(seed),
                ..Default::default()
            };
            ruqu_core::simulator::Simulator::run_with_config(&c, &config)
                .unwrap()
                .measurements[0]
                .result
        };
        let outcomes: Vec<bool> = (0..32).map(run).collect();
        assert_eq!(outcomes, (0..32).map(run).collect::<Vec<_>>());
        assert!(outcomes.contains(&true) && outcomes.contains(&false));
    }
}

/// Execute the same bounded research JSON API used by native Rust and the CLI.
/// Errors (including exhausted exact-computation budgets) never return partial
/// or approximate numerical results. No browser networking is performed.
#[wasm_bindgen]
pub fn research_run_json(request: &str) -> Result<String, JsValue> {
    ruqu_core::research_api::run_json(request).map_err(|e| JsValue::from_str(&e))
}
