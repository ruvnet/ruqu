/* tslint:disable */
/* eslint-disable */

/**
 * A JavaScript-friendly quantum circuit builder.
 *
 * Wraps `ruqu_core::circuit::QuantumCircuit` with wasm-bindgen annotations.
 * All gate methods validate qubit indices against the circuit size internally
 * via the core library.
 *
 * ## JavaScript Example
 *
 * ```javascript
 * const qc = new WasmQuantumCircuit(3);
 * qc.h(0);           // Hadamard on qubit 0
 * qc.cnot(0, 1);     // CNOT: control=0, target=1
 * qc.rz(2, Math.PI); // Rz rotation on qubit 2
 * qc.measure_all();
 *
 * console.log(`Qubits: ${qc.num_qubits}`);
 * console.log(`Gates:  ${qc.gate_count}`);
 * console.log(`Depth:  ${qc.depth}`);
 * ```
 */
export class WasmQuantumCircuit {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Insert a barrier (prevents gate reordering across this point).
     */
    barrier(): void;
    /**
     * Apply CNOT (controlled-X) gate.
     */
    cnot(control: number, target: number): void;
    /**
     * Apply controlled-Z gate.
     */
    cz(q1: number, q2: number): void;
    /**
     * Apply Hadamard gate to the target qubit.
     */
    h(qubit: number): void;
    /**
     * Add a measurement operation on a single qubit.
     */
    measure(qubit: number): void;
    /**
     * Add measurement operations on all qubits.
     */
    measure_all(): void;
    /**
     * Create a new quantum circuit with the given number of qubits.
     *
     * Returns an error if `num_qubits` exceeds the WASM limit (25).
     */
    constructor(num_qubits: number);
    /**
     * Reset a qubit to the |0> state.
     */
    reset(qubit: number): void;
    /**
     * Apply Rx rotation gate with the given angle (radians).
     */
    rx(qubit: number, angle: number): void;
    /**
     * Apply Ry rotation gate with the given angle (radians).
     */
    ry(qubit: number, angle: number): void;
    /**
     * Apply Rz rotation gate with the given angle (radians).
     */
    rz(qubit: number, angle: number): void;
    /**
     * Apply Rzz (ZZ-rotation) gate with the given angle (radians).
     */
    rzz(q1: number, q2: number, angle: number): void;
    /**
     * Apply S (phase) gate to the target qubit.
     */
    s(qubit: number): void;
    /**
     * Apply SWAP gate.
     */
    swap(q1: number, q2: number): void;
    /**
     * Apply T gate to the target qubit.
     */
    t(qubit: number): void;
    /**
     * Apply Pauli-X (NOT) gate to the target qubit.
     */
    x(qubit: number): void;
    /**
     * Apply Pauli-Y gate to the target qubit.
     */
    y(qubit: number): void;
    /**
     * Apply Pauli-Z gate to the target qubit.
     */
    z(qubit: number): void;
    /**
     * The circuit depth (longest path through the gate DAG).
     */
    readonly depth: number;
    /**
     * The total number of gates applied so far.
     */
    readonly gate_count: number;
    /**
     * The number of qubits in this circuit.
     */
    readonly num_qubits: number;
}

/**
 * Estimate memory usage (in bytes) for a state vector of `num_qubits` qubits.
 *
 * Each qubit doubles the state vector size. The formula is `2^n * 16` bytes
 * (two f64 values per complex amplitude).
 */
export function estimate_memory(num_qubits: number): number;

/**
 * Run Grover's quantum search algorithm.
 *
 * Searches for one or more target states in a space of `2^num_qubits` items.
 * The optimal number of iterations is computed automatically when not specified.
 *
 * ## Parameters
 *
 * - `num_qubits` - Number of qubits (search space = 2^num_qubits).
 * - `target_states` - Array of target state indices to search for (as u32 values).
 * - `seed` - Optional RNG seed for reproducibility. Pass `null` or `undefined`
 *            for non-deterministic execution. If provided, interpreted as a
 *            floating-point number and truncated to a 64-bit unsigned integer.
 *
 * ## Returns
 *
 * A JS object:
 * ```typescript
 * {
 *   measured_state: number,
 *   target_found: boolean,
 *   success_probability: number,
 *   num_iterations: number,
 * }
 * ```
 */
export function grover_search(num_qubits: number, target_states: Uint32Array, seed: any): any;

/**
 * Called automatically when the WASM module is instantiated.
 *
 * Sets up `console_error_panic_hook` (when the feature is enabled) so that
 * Rust panics produce readable stack traces in the browser console instead
 * of opaque "unreachable" errors.
 */
export function init(): void;

/**
 * Get the maximum number of qubits supported in the WASM environment.
 */
export function max_qubits(): number;

/**
 * Build and simulate a QAOA (Quantum Approximate Optimization Algorithm)
 * circuit for the MaxCut problem on an undirected graph.
 *
 * ## Parameters
 *
 * - `num_nodes` - Number of graph nodes (one qubit per node).
 * - `edges_flat` - Flattened edge list as consecutive pairs: `[i1, j1, i2, j2, ...]`.
 *   Each `(i, j)` pair defines an undirected edge with unit weight.
 * - `p` - Number of QAOA rounds (circuit depth parameter).
 * - `gammas` - Problem-unitary angles, length must equal `p`.
 * - `betas` - Mixer-unitary angles, length must equal `p`.
 * - `seed` - Optional RNG seed. Pass `null` or `undefined` for non-deterministic
 *            execution.
 *
 * ## Returns
 *
 * A JS object:
 * ```typescript
 * {
 *   probabilities: number[],   // length = 2^num_nodes
 *   expected_cut: number,      // expected cut value from the output state
 * }
 * ```
 */
export function qaoa_maxcut(num_nodes: number, edges_flat: Uint32Array, p: number, gammas: Float64Array, betas: Float64Array, seed: any): any;

/**
 * Run a quantum circuit simulation and return the results as a JS object.
 *
 * The returned object has the shape:
 * ```typescript
 * {
 *   probabilities: number[],   // length = 2^num_qubits
 *   measurements: Array<{ qubit: number, result: boolean, probability: number }>,
 *   num_qubits: number,
 *   gate_count: number,
 *   execution_time_ms: number,
 * }
 * ```
 */
export function simulate(circuit: WasmQuantumCircuit): any;
