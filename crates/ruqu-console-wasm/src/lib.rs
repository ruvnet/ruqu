//! # ruqu-console-wasm — WebAssembly bindings for the management console
//!
//! Browser-facing API for the ruQu **Structural Possibility Runtime** (ADR-258).
//! A single WASM module that drives the whole runtime client-side — possibility
//! fields, interference reranking (RAG), swarm collapse consensus, structural
//! sensing/diagnosis, a live syndrome→gate path for WebSocket feeds, an
//! auditable receipt log, and a couple of quantum-circuit demos.
//!
//! Every function takes JSON strings (so callers just `JSON.stringify` their
//! inputs) and returns plain JS objects via `serde-wasm-bindgen`. Decisions and
//! receipts come back exactly as the native runtime emits them, so the UI is a
//! thin view over the real Rust logic.
//!
//! Build for the browser with:
//!
//! ```bash
//! wasm-pack build crates/ruqu-console-wasm --target web
//! ```

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use ruqu_possibility::{CoherenceGate, CollapseReceipt, Possibility, PossibilityField};
use ruqu_rag::{QuantumRagIndex, RagCandidate};
use ruqu_receipts::ReceiptStore;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn to_js<T: Serialize>(v: &T) -> Result<JsValue, JsValue> {
    serde_wasm_bindgen::to_value(v).map_err(|e| JsValue::from_str(&e.to_string()))
}

fn from_json<T: for<'de> Deserialize<'de>>(s: &str) -> Result<T, JsValue> {
    serde_json::from_str(s).map_err(|e| JsValue::from_str(&format!("invalid JSON: {e}")))
}

fn err(e: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&e.to_string())
}

fn one() -> f64 {
    1.0
}

// ---------------------------------------------------------------------------
// Init + version
// ---------------------------------------------------------------------------

/// Called automatically when the module is instantiated; installs a panic hook
/// so Rust panics surface as readable console errors.
#[wasm_bindgen(start)]
pub fn init() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

/// Runtime version banner: `{ console, possibility, sensing, receipts }`.
#[wasm_bindgen]
pub fn version() -> Result<JsValue, JsValue> {
    #[derive(Serialize)]
    struct V {
        console: &'static str,
        possibility: &'static str,
        sensing: &'static str,
        receipts: &'static str,
    }
    to_js(&V {
        console: env!("CARGO_PKG_VERSION"),
        possibility: ruqu_possibility::VERSION,
        sensing: ruqu_sensing::VERSION,
        receipts: ruqu_receipts::VERSION,
    })
}

// ---------------------------------------------------------------------------
// Possibility field
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct FieldCandidateInput {
    id: String,
    amplitude: f64,
    phase: f64,
}

/// Analyze a possibility field and (unless empty) collapse it to its argmax.
///
/// Input: `candidates_json` = `[{ "id", "amplitude", "phase" }, ...]`.
/// Output:
/// ```typescript
/// {
///   entropy: number, coherence: number, gate: "PERMIT"|"DEFER"|"DENY",
///   field_hash: string, probabilities: number[],
///   selected_id: string | null, receipt: CollapseReceipt | null
/// }
/// ```
#[wasm_bindgen]
pub fn analyze_field(candidates_json: &str, seed: f64) -> Result<JsValue, JsValue> {
    let inputs: Vec<FieldCandidateInput> = from_json(candidates_json)?;
    let field = PossibilityField::new(
        inputs
            .into_iter()
            .map(|c| Possibility::new(c.id.clone(), c.id, c.amplitude, c.phase))
            .collect(),
    );

    let gate = CoherenceGate::with_defaults();
    let decision = gate.evaluate(&field);

    let (selected_id, receipt) = if field.is_empty() {
        (None, None)
    } else {
        let (sel, rcpt) = field
            .collapse_argmax_with_gate(seed as u64, &gate)
            .map_err(err)?;
        (Some(sel.id), Some(rcpt))
    };

    #[derive(Serialize)]
    struct Out {
        entropy: f64,
        coherence: f64,
        gate: &'static str,
        field_hash: String,
        probabilities: Vec<f64>,
        selected_id: Option<String>,
        receipt: Option<CollapseReceipt>,
    }
    to_js(&Out {
        entropy: field.entropy(),
        coherence: field.coherence(),
        gate: decision.as_str(),
        field_hash: if field.is_empty() {
            String::new()
        } else {
            field.field_hash()
        },
        probabilities: field.probabilities(),
        selected_id,
        receipt,
    })
}

// ---------------------------------------------------------------------------
// Interference RAG
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct RagDocInput {
    id: String,
    text: String,
    embedding: Vec<f64>,
    #[serde(default = "one")]
    source_trust: f64,
    #[serde(default = "one")]
    recency: f64,
    #[serde(default = "one")]
    graph_proximity: f64,
    #[serde(default)]
    contradiction: f64,
    #[serde(default)]
    novelty: f64,
}

/// Run interference-reranked retrieval and compare it to plain cosine.
///
/// Inputs: `query_json` = `number[]`; `corpus_json` = array of
/// `{ id, text, embedding, source_trust?, recency?, graph_proximity?,
/// contradiction?, novelty? }`.
/// Output:
/// ```typescript
/// {
///   selected: Array<{ id, text, score, phase }>,  // interference top-k
///   cosine_top_k: string[],                        // plain cosine baseline ids
///   gate: "PERMIT"|"DEFER"|"DENY",
///   receipt: CollapseReceipt
/// }
/// ```
#[wasm_bindgen]
pub fn rag_search(
    query_json: &str,
    corpus_json: &str,
    k: usize,
    rounds: usize,
    phase_kickback: bool,
    seed: f64,
) -> Result<JsValue, JsValue> {
    let query: Vec<f64> = from_json(query_json)?;
    let docs: Vec<RagDocInput> = from_json(corpus_json)?;
    if docs.is_empty() {
        return Err(JsValue::from_str("corpus is empty"));
    }

    let mut index = QuantumRagIndex::new(query.len())
        .interference_rounds(rounds)
        .phase_kickback(phase_kickback);
    for d in docs {
        index.add(
            RagCandidate::new(d.id, d.text, d.embedding)
                .with_source_trust(d.source_trust)
                .with_recency(d.recency)
                .with_graph_proximity(d.graph_proximity)
                .with_contradiction(d.contradiction)
                .with_novelty(d.novelty),
        );
    }

    let result = index.search(&query, k, seed as u64).map_err(err)?;

    #[derive(Serialize)]
    struct ScoredOut {
        id: String,
        text: String,
        score: f64,
        phase: f64,
    }
    #[derive(Serialize)]
    struct Out {
        selected: Vec<ScoredOut>,
        cosine_top_k: Vec<String>,
        gate: &'static str,
        receipt: CollapseReceipt,
    }
    to_js(&Out {
        selected: result
            .selected
            .into_iter()
            .map(|s| ScoredOut {
                id: s.id,
                text: s.text,
                score: s.score,
                phase: s.phase,
            })
            .collect(),
        cosine_top_k: result.baseline_cosine_top_k,
        gate: result.gate.as_str(),
        receipt: result.receipt,
    })
}

// ---------------------------------------------------------------------------
// Swarm collapse consensus
// ---------------------------------------------------------------------------

/// Run swarm collapse consensus over a serialized [`ruqu_agent::AgentWavefront`].
///
/// `wavefront_json` matches the `AgentWavefront` shape:
/// `{ agents:[{id,role,confidence}], plans:[{id,description,evidence_support,steps}],
///    votes:[{agent_id,plan_id,confidence,support}], consensus_threshold }`.
/// Returns a `ConsensusOutcome` (with the collapse receipt + reasoning-QEC
/// verifier attached).
#[wasm_bindgen]
pub fn swarm_consensus(wavefront_json: &str, seed: f64) -> Result<JsValue, JsValue> {
    let wavefront: ruqu_agent::AgentWavefront = from_json(wavefront_json)?;
    let outcome = wavefront.coordinate(seed as u64).map_err(err)?;
    to_js(&outcome)
}

// ---------------------------------------------------------------------------
// Sensing: structural diagnosis + live syndrome→gate
// ---------------------------------------------------------------------------

/// Run structural syndrome diagnosis over a serialized
/// [`ruqu_sensing::SystemTopology`].
///
/// `topology_json` = `{ components:[string], health:[number], connections:[[from,to,strength]] }`.
/// Returns a `SystemDiagnosis` (`{ fragility_scores, weakest_component,
/// fault_propagators, severity }`).
#[wasm_bindgen]
pub fn sensing_diagnose(
    topology_json: &str,
    fault_rate: f64,
    rounds: usize,
    seed: f64,
) -> Result<JsValue, JsValue> {
    let topology: ruqu_sensing::SystemTopology = from_json(topology_json)?;
    let diagnosis = topology
        .diagnose(fault_rate, rounds, seed as u64)
        .map_err(err)?;
    to_js(&diagnosis)
}

/// The live path for a WebSocket telemetry feed: turn a batch of
/// [`ruqu_sensing::SensorSyndrome`]s into a coherence-gated root-cause decision.
///
/// Inputs: `syndromes_json` = array of `SensorSyndrome`
/// (`{ source, detector_bits:[bool], confidence, timestamp_ns }`);
/// `labels_json` = component labels (`string[]`, one per detector bit).
/// Output:
/// ```typescript
/// {
///   coherence, entropy, gate, field_hash,
///   root_cause: string | null, probabilities: number[],
///   receipt: CollapseReceipt | null
/// }
/// ```
#[wasm_bindgen]
pub fn gate_from_syndromes(
    syndromes_json: &str,
    labels_json: &str,
    seed: f64,
) -> Result<JsValue, JsValue> {
    let syndromes: Vec<ruqu_sensing::SensorSyndrome> = from_json(syndromes_json)?;
    let labels: Vec<String> = from_json(labels_json)?;
    let field = ruqu_sensing::fault_field(&syndromes, &labels);

    let gate = CoherenceGate::with_defaults();
    let decision = gate.evaluate(&field);
    let root_cause = field.argmax().map(|p| p.id.clone());

    let (field_hash, receipt) = if field.is_empty() {
        (String::new(), None)
    } else {
        let (_sel, rcpt) = field
            .collapse_argmax_with_gate(seed as u64, &gate)
            .map_err(err)?;
        (field.field_hash(), Some(rcpt))
    };

    #[derive(Serialize)]
    struct Out {
        coherence: f64,
        entropy: f64,
        gate: &'static str,
        field_hash: String,
        root_cause: Option<String>,
        probabilities: Vec<f64>,
        receipt: Option<CollapseReceipt>,
    }
    to_js(&Out {
        coherence: field.coherence(),
        entropy: field.entropy(),
        gate: decision.as_str(),
        field_hash,
        root_cause,
        probabilities: field.probabilities(),
        receipt,
    })
}

/// Build [`ruqu_sensing::SensorSyndrome`]s from raw channel samples — useful for
/// the simulated feed and for normalizing a raw telemetry stream client-side.
///
/// Inputs: `channels_json` = `[{ id, threshold }]`;
/// `samples_json` = `[[timestamp_ns, [values...]]]`.
/// Returns `SensorSyndrome[]`.
#[wasm_bindgen]
pub fn syndromes_from_samples(
    channels_json: &str,
    samples_json: &str,
) -> Result<JsValue, JsValue> {
    let channels: Vec<ruqu_sensing::SensorChannel> = from_json(channels_json)?;
    let samples: Vec<(u64, Vec<f64>)> = from_json(samples_json)?;
    let syndromes = ruqu_sensing::syndromes_from_samples(&channels, &samples).map_err(err)?;
    to_js(&syndromes)
}

// ---------------------------------------------------------------------------
// Receipt audit log (stateful, hash-chained)
// ---------------------------------------------------------------------------

/// A tamper-evident, hash-chained receipt log the UI can append collapses to and
/// verify — a thin wrapper over [`ruqu_receipts::ReceiptStore`].
#[wasm_bindgen]
pub struct WasmReceiptLog {
    store: ReceiptStore,
}

#[wasm_bindgen]
impl WasmReceiptLog {
    /// Create an empty log.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            store: ReceiptStore::new(),
        }
    }

    /// Append a `CollapseReceipt` (as JSON) and return its chain `entry_hash`.
    pub fn append(&mut self, receipt_json: &str) -> Result<String, JsValue> {
        let receipt: CollapseReceipt = from_json(receipt_json)?;
        let entry = self.store.append(receipt);
        Ok(entry.entry_hash.clone())
    }

    /// Number of entries in the chain.
    pub fn len(&self) -> usize {
        self.store.len()
    }

    /// Whether the chain is empty.
    pub fn is_empty(&self) -> bool {
        self.store.is_empty()
    }

    /// Re-derive and verify chain integrity.
    pub fn verify(&self) -> bool {
        self.store.verify_chain()
    }

    /// The current chain tip hash.
    pub fn tip(&self) -> String {
        self.store.tip_hash()
    }

    /// Export the whole log as JSON Lines.
    pub fn to_jsonl(&self) -> String {
        self.store.to_jsonl()
    }
}

impl Default for WasmReceiptLog {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Quantum-circuit demos (single bundle convenience)
// ---------------------------------------------------------------------------

/// Simulate an `n`-qubit GHZ state and return the basis-state probability
/// distribution (length `2^n`). A small, visual "the quantum engine is live"
/// demo for the console.
#[wasm_bindgen]
pub fn quantum_ghz(num_qubits: u32) -> Result<JsValue, JsValue> {
    if num_qubits == 0 || num_qubits > 16 {
        return Err(JsValue::from_str("num_qubits must be in 1..=16 for the demo"));
    }
    let mut circuit = ruqu_core::circuit::QuantumCircuit::new(num_qubits);
    circuit.h(0);
    for q in 1..num_qubits {
        circuit.cnot(0, q);
    }
    let result = ruqu_core::simulator::Simulator::run(&circuit).map_err(err)?;

    #[derive(Serialize)]
    struct Out {
        num_qubits: u32,
        probabilities: Vec<f64>,
    }
    to_js(&Out {
        num_qubits,
        probabilities: result.state.probabilities(),
    })
}

/// Run Grover search for a single target in a `2^num_qubits` space; returns
/// `{ measured_state, target_found, success_probability, num_iterations }`.
#[wasm_bindgen]
pub fn quantum_grover(num_qubits: u32, target: u32, seed: f64) -> Result<JsValue, JsValue> {
    if num_qubits == 0 || num_qubits > 16 {
        return Err(JsValue::from_str("num_qubits must be in 1..=16 for the demo"));
    }
    let config = ruqu_algorithms::grover::GroverConfig {
        num_qubits,
        target_states: vec![target as usize],
        num_iterations: None,
        seed: Some(seed as u64),
    };
    let result = ruqu_algorithms::grover::run_grover(&config).map_err(err)?;

    #[derive(Serialize)]
    struct Out {
        measured_state: usize,
        target_found: bool,
        success_probability: f64,
        num_iterations: u32,
    }
    to_js(&Out {
        measured_state: result.measured_state,
        target_found: result.target_found,
        success_probability: result.success_probability,
        num_iterations: result.num_iterations,
    })
}
