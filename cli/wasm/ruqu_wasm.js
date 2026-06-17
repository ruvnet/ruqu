/* @ts-self-types="./ruqu_wasm.d.ts" */

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
class WasmQuantumCircuit {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmQuantumCircuitFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmquantumcircuit_free(ptr, 0);
    }
    /**
     * Insert a barrier (prevents gate reordering across this point).
     */
    barrier() {
        wasm.wasmquantumcircuit_barrier(this.__wbg_ptr);
    }
    /**
     * Apply CNOT (controlled-X) gate.
     * @param {number} control
     * @param {number} target
     */
    cnot(control, target) {
        wasm.wasmquantumcircuit_cnot(this.__wbg_ptr, control, target);
    }
    /**
     * Apply controlled-Z gate.
     * @param {number} q1
     * @param {number} q2
     */
    cz(q1, q2) {
        wasm.wasmquantumcircuit_cz(this.__wbg_ptr, q1, q2);
    }
    /**
     * The circuit depth (longest path through the gate DAG).
     * @returns {number}
     */
    get depth() {
        const ret = wasm.wasmquantumcircuit_depth(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * The total number of gates applied so far.
     * @returns {number}
     */
    get gate_count() {
        const ret = wasm.wasmquantumcircuit_gate_count(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Apply Hadamard gate to the target qubit.
     * @param {number} qubit
     */
    h(qubit) {
        wasm.wasmquantumcircuit_h(this.__wbg_ptr, qubit);
    }
    /**
     * Add a measurement operation on a single qubit.
     * @param {number} qubit
     */
    measure(qubit) {
        wasm.wasmquantumcircuit_measure(this.__wbg_ptr, qubit);
    }
    /**
     * Add measurement operations on all qubits.
     */
    measure_all() {
        wasm.wasmquantumcircuit_measure_all(this.__wbg_ptr);
    }
    /**
     * Create a new quantum circuit with the given number of qubits.
     *
     * Returns an error if `num_qubits` exceeds the WASM limit (25).
     * @param {number} num_qubits
     */
    constructor(num_qubits) {
        const ret = wasm.wasmquantumcircuit_new(num_qubits);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        this.__wbg_ptr = ret[0];
        WasmQuantumCircuitFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * The number of qubits in this circuit.
     * @returns {number}
     */
    get num_qubits() {
        const ret = wasm.wasmquantumcircuit_num_qubits(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Reset a qubit to the |0> state.
     * @param {number} qubit
     */
    reset(qubit) {
        wasm.wasmquantumcircuit_reset(this.__wbg_ptr, qubit);
    }
    /**
     * Apply Rx rotation gate with the given angle (radians).
     * @param {number} qubit
     * @param {number} angle
     */
    rx(qubit, angle) {
        wasm.wasmquantumcircuit_rx(this.__wbg_ptr, qubit, angle);
    }
    /**
     * Apply Ry rotation gate with the given angle (radians).
     * @param {number} qubit
     * @param {number} angle
     */
    ry(qubit, angle) {
        wasm.wasmquantumcircuit_ry(this.__wbg_ptr, qubit, angle);
    }
    /**
     * Apply Rz rotation gate with the given angle (radians).
     * @param {number} qubit
     * @param {number} angle
     */
    rz(qubit, angle) {
        wasm.wasmquantumcircuit_rz(this.__wbg_ptr, qubit, angle);
    }
    /**
     * Apply Rzz (ZZ-rotation) gate with the given angle (radians).
     * @param {number} q1
     * @param {number} q2
     * @param {number} angle
     */
    rzz(q1, q2, angle) {
        wasm.wasmquantumcircuit_rzz(this.__wbg_ptr, q1, q2, angle);
    }
    /**
     * Apply S (phase) gate to the target qubit.
     * @param {number} qubit
     */
    s(qubit) {
        wasm.wasmquantumcircuit_s(this.__wbg_ptr, qubit);
    }
    /**
     * Apply SWAP gate.
     * @param {number} q1
     * @param {number} q2
     */
    swap(q1, q2) {
        wasm.wasmquantumcircuit_swap(this.__wbg_ptr, q1, q2);
    }
    /**
     * Apply T gate to the target qubit.
     * @param {number} qubit
     */
    t(qubit) {
        wasm.wasmquantumcircuit_t(this.__wbg_ptr, qubit);
    }
    /**
     * Apply Pauli-X (NOT) gate to the target qubit.
     * @param {number} qubit
     */
    x(qubit) {
        wasm.wasmquantumcircuit_x(this.__wbg_ptr, qubit);
    }
    /**
     * Apply Pauli-Y gate to the target qubit.
     * @param {number} qubit
     */
    y(qubit) {
        wasm.wasmquantumcircuit_y(this.__wbg_ptr, qubit);
    }
    /**
     * Apply Pauli-Z gate to the target qubit.
     * @param {number} qubit
     */
    z(qubit) {
        wasm.wasmquantumcircuit_z(this.__wbg_ptr, qubit);
    }
}
if (Symbol.dispose) WasmQuantumCircuit.prototype[Symbol.dispose] = WasmQuantumCircuit.prototype.free;
exports.WasmQuantumCircuit = WasmQuantumCircuit;

/**
 * Estimate memory usage (in bytes) for a state vector of `num_qubits` qubits.
 *
 * Each qubit doubles the state vector size. The formula is `2^n * 16` bytes
 * (two f64 values per complex amplitude).
 * @param {number} num_qubits
 * @returns {number}
 */
function estimate_memory(num_qubits) {
    const ret = wasm.estimate_memory(num_qubits);
    return ret >>> 0;
}
exports.estimate_memory = estimate_memory;

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
 * @param {number} num_qubits
 * @param {Uint32Array} target_states
 * @param {any} seed
 * @returns {any}
 */
function grover_search(num_qubits, target_states, seed) {
    const ptr0 = passArray32ToWasm0(target_states, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.grover_search(num_qubits, ptr0, len0, seed);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}
exports.grover_search = grover_search;

/**
 * Called automatically when the WASM module is instantiated.
 *
 * Sets up `console_error_panic_hook` (when the feature is enabled) so that
 * Rust panics produce readable stack traces in the browser console instead
 * of opaque "unreachable" errors.
 */
function init() {
    wasm.init();
}
exports.init = init;

/**
 * Get the maximum number of qubits supported in the WASM environment.
 * @returns {number}
 */
function max_qubits() {
    const ret = wasm.max_qubits();
    return ret >>> 0;
}
exports.max_qubits = max_qubits;

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
 * @param {number} num_nodes
 * @param {Uint32Array} edges_flat
 * @param {number} p
 * @param {Float64Array} gammas
 * @param {Float64Array} betas
 * @param {any} seed
 * @returns {any}
 */
function qaoa_maxcut(num_nodes, edges_flat, p, gammas, betas, seed) {
    const ptr0 = passArray32ToWasm0(edges_flat, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passArrayF64ToWasm0(gammas, wasm.__wbindgen_malloc);
    const len1 = WASM_VECTOR_LEN;
    const ptr2 = passArrayF64ToWasm0(betas, wasm.__wbindgen_malloc);
    const len2 = WASM_VECTOR_LEN;
    const ret = wasm.qaoa_maxcut(num_nodes, ptr0, len0, p, ptr1, len1, ptr2, len2, seed);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}
exports.qaoa_maxcut = qaoa_maxcut;

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
 * @param {WasmQuantumCircuit} circuit
 * @returns {any}
 */
function simulate(circuit) {
    _assertClass(circuit, WasmQuantumCircuit);
    const ret = wasm.simulate(circuit.__wbg_ptr);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}
exports.simulate = simulate;
function __wbg_get_imports() {
    const import0 = {
        __proto__: null,
        __wbg_Error_fdd633d4bb5dd76a: function(arg0, arg1) {
            const ret = Error(getStringFromWasm0(arg0, arg1));
            return ret;
        },
        __wbg_String_8564e559799eccda: function(arg0, arg1) {
            const ret = String(arg1);
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg___wbindgen_is_function_acc5528be2b923f2: function(arg0) {
            const ret = typeof(arg0) === 'function';
            return ret;
        },
        __wbg___wbindgen_is_null_6d937fbfb6478470: function(arg0) {
            const ret = arg0 === null;
            return ret;
        },
        __wbg___wbindgen_is_object_0beba4a1980d3eea: function(arg0) {
            const val = arg0;
            const ret = typeof(val) === 'object' && val !== null;
            return ret;
        },
        __wbg___wbindgen_is_string_1fca8072260dd261: function(arg0) {
            const ret = typeof(arg0) === 'string';
            return ret;
        },
        __wbg___wbindgen_is_undefined_721f8decd50c87a3: function(arg0) {
            const ret = arg0 === undefined;
            return ret;
        },
        __wbg___wbindgen_number_get_1cc01dd708740256: function(arg0, arg1) {
            const obj = arg1;
            const ret = typeof(obj) === 'number' ? obj : undefined;
            getDataViewMemory0().setFloat64(arg0 + 8 * 1, isLikeNone(ret) ? 0 : ret, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, !isLikeNone(ret), true);
        },
        __wbg___wbindgen_throw_ea4887a5f8f9a9db: function(arg0, arg1) {
            throw new Error(getStringFromWasm0(arg0, arg1));
        },
        __wbg_call_5575218572ead796: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.call(arg1, arg2);
            return ret;
        }, arguments); },
        __wbg_crypto_38df2bab126b63dc: function(arg0) {
            const ret = arg0.crypto;
            return ret;
        },
        __wbg_getRandomValues_c44a50d8cfdaebeb: function() { return handleError(function (arg0, arg1) {
            arg0.getRandomValues(arg1);
        }, arguments); },
        __wbg_length_589238bdcf171f0e: function(arg0) {
            const ret = arg0.length;
            return ret;
        },
        __wbg_msCrypto_bd5a034af96bcba6: function(arg0) {
            const ret = arg0.msCrypto;
            return ret;
        },
        __wbg_new_2e117a478906f062: function() {
            const ret = new Object();
            return ret;
        },
        __wbg_new_36e147a8ced3c6e0: function() {
            const ret = new Array();
            return ret;
        },
        __wbg_new_with_length_9b650f44b5c44a4e: function(arg0) {
            const ret = new Uint8Array(arg0 >>> 0);
            return ret;
        },
        __wbg_node_84ea875411254db1: function(arg0) {
            const ret = arg0.node;
            return ret;
        },
        __wbg_now_e7c6795a7f81e10f: function(arg0) {
            const ret = arg0.now();
            return ret;
        },
        __wbg_performance_3fcf6e32a7e1ed0a: function(arg0) {
            const ret = arg0.performance;
            return ret;
        },
        __wbg_process_44c7a14e11e9f69e: function(arg0) {
            const ret = arg0.process;
            return ret;
        },
        __wbg_prototypesetcall_d721637c7ca66eb8: function(arg0, arg1, arg2) {
            Uint8Array.prototype.set.call(getArrayU8FromWasm0(arg0, arg1), arg2);
        },
        __wbg_randomFillSync_6c25eac9869eb53c: function() { return handleError(function (arg0, arg1) {
            arg0.randomFillSync(arg1);
        }, arguments); },
        __wbg_require_b4edbdcf3e2a1ef0: function() { return handleError(function () {
            const ret = module.require;
            return ret;
        }, arguments); },
        __wbg_set_6be42768c690e380: function(arg0, arg1, arg2) {
            arg0[arg1] = arg2;
        },
        __wbg_set_dc601f4a69da0bc2: function(arg0, arg1, arg2) {
            arg0[arg1 >>> 0] = arg2;
        },
        __wbg_static_accessor_GLOBAL_THIS_2fee5048bcca5938: function() {
            const ret = typeof globalThis === 'undefined' ? null : globalThis;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_GLOBAL_ce44e66a4935da8c: function() {
            const ret = typeof global === 'undefined' ? null : global;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_SELF_44f6e0cb5e67cdad: function() {
            const ret = typeof self === 'undefined' ? null : self;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_WINDOW_168f178805d978fe: function() {
            const ret = typeof window === 'undefined' ? null : window;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_subarray_b0e8ac4ed313fea8: function(arg0, arg1, arg2) {
            const ret = arg0.subarray(arg1 >>> 0, arg2 >>> 0);
            return ret;
        },
        __wbg_versions_276b2795b1c6a219: function(arg0) {
            const ret = arg0.versions;
            return ret;
        },
        __wbindgen_cast_0000000000000001: function(arg0) {
            // Cast intrinsic for `F64 -> Externref`.
            const ret = arg0;
            return ret;
        },
        __wbindgen_cast_0000000000000002: function(arg0, arg1) {
            // Cast intrinsic for `Ref(Slice(U8)) -> NamedExternref("Uint8Array")`.
            const ret = getArrayU8FromWasm0(arg0, arg1);
            return ret;
        },
        __wbindgen_cast_0000000000000003: function(arg0, arg1) {
            // Cast intrinsic for `Ref(String) -> Externref`.
            const ret = getStringFromWasm0(arg0, arg1);
            return ret;
        },
        __wbindgen_cast_0000000000000004: function(arg0) {
            // Cast intrinsic for `U64 -> Externref`.
            const ret = BigInt.asUintN(64, arg0);
            return ret;
        },
        __wbindgen_init_externref_table: function() {
            const table = wasm.__wbindgen_externrefs;
            const offset = table.grow(4);
            table.set(0, undefined);
            table.set(offset + 0, undefined);
            table.set(offset + 1, null);
            table.set(offset + 2, true);
            table.set(offset + 3, false);
        },
    };
    return {
        __proto__: null,
        "./ruqu_wasm_bg.js": import0,
    };
}

const WasmQuantumCircuitFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmquantumcircuit_free(ptr, 1));

function addToExternrefTable0(obj) {
    const idx = wasm.__externref_table_alloc();
    wasm.__wbindgen_externrefs.set(idx, obj);
    return idx;
}

function _assertClass(instance, klass) {
    if (!(instance instanceof klass)) {
        throw new Error(`expected instance of ${klass.name}`);
    }
}

function getArrayU8FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint8ArrayMemory0().subarray(ptr / 1, ptr / 1 + len);
}

let cachedDataViewMemory0 = null;
function getDataViewMemory0() {
    if (cachedDataViewMemory0 === null || cachedDataViewMemory0.buffer.detached === true || (cachedDataViewMemory0.buffer.detached === undefined && cachedDataViewMemory0.buffer !== wasm.memory.buffer)) {
        cachedDataViewMemory0 = new DataView(wasm.memory.buffer);
    }
    return cachedDataViewMemory0;
}

let cachedFloat64ArrayMemory0 = null;
function getFloat64ArrayMemory0() {
    if (cachedFloat64ArrayMemory0 === null || cachedFloat64ArrayMemory0.byteLength === 0) {
        cachedFloat64ArrayMemory0 = new Float64Array(wasm.memory.buffer);
    }
    return cachedFloat64ArrayMemory0;
}

function getStringFromWasm0(ptr, len) {
    return decodeText(ptr >>> 0, len);
}

let cachedUint32ArrayMemory0 = null;
function getUint32ArrayMemory0() {
    if (cachedUint32ArrayMemory0 === null || cachedUint32ArrayMemory0.byteLength === 0) {
        cachedUint32ArrayMemory0 = new Uint32Array(wasm.memory.buffer);
    }
    return cachedUint32ArrayMemory0;
}

let cachedUint8ArrayMemory0 = null;
function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

function handleError(f, args) {
    try {
        return f.apply(this, args);
    } catch (e) {
        const idx = addToExternrefTable0(e);
        wasm.__wbindgen_exn_store(idx);
    }
}

function isLikeNone(x) {
    return x === undefined || x === null;
}

function passArray32ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 4, 4) >>> 0;
    getUint32ArrayMemory0().set(arg, ptr / 4);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
}

function passArrayF64ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 8, 8) >>> 0;
    getFloat64ArrayMemory0().set(arg, ptr / 8);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
}

function passStringToWasm0(arg, malloc, realloc) {
    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length, 1) >>> 0;
        getUint8ArrayMemory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len, 1) >>> 0;

    const mem = getUint8ArrayMemory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }
    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3, 1) >>> 0;
        const view = getUint8ArrayMemory0().subarray(ptr + offset, ptr + len);
        const ret = cachedTextEncoder.encodeInto(arg, view);

        offset += ret.written;
        ptr = realloc(ptr, len, offset, 1) >>> 0;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

function takeFromExternrefTable0(idx) {
    const value = wasm.__wbindgen_externrefs.get(idx);
    wasm.__externref_table_dealloc(idx);
    return value;
}

let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
cachedTextDecoder.decode();
function decodeText(ptr, len) {
    return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}

const cachedTextEncoder = new TextEncoder();

if (!('encodeInto' in cachedTextEncoder)) {
    cachedTextEncoder.encodeInto = function (arg, view) {
        const buf = cachedTextEncoder.encode(arg);
        view.set(buf);
        return {
            read: arg.length,
            written: buf.length
        };
    };
}

let WASM_VECTOR_LEN = 0;

const wasmPath = `${__dirname}/ruqu_wasm_bg.wasm`;
const wasmBytes = require('fs').readFileSync(wasmPath);
const wasmModule = new WebAssembly.Module(wasmBytes);
let wasmInstance = new WebAssembly.Instance(wasmModule, __wbg_get_imports());
let wasm = wasmInstance.exports;
wasm.__wbindgen_start();
