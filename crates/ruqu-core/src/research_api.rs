//! Bounded JSON research interface shared by native clients and WebAssembly.
//! All numerical operations execute the same audited Rust implementations.
use crate::magic_states::*;
use serde::Deserialize;
use serde_json::{json, Value};

pub const MAX_REQUEST_BYTES: usize = 1_048_576;

#[derive(Deserialize)]
struct Header {
    op: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FactoryRequest {
    op: String,
    #[serde(default = "default_error")]
    input_error: f64,
    #[serde(default = "default_shots")]
    shots: u64,
    #[serde(default = "default_seed")]
    seed: String,
    #[serde(default = "default_rounds")]
    rounds: usize,
    #[serde(default = "block_qubits")]
    block_qubits: f64,
    #[serde(default = "block_rounds")]
    block_rounds: f64,
    #[serde(default = "one")]
    raw_qubits: f64,
    #[serde(default = "one")]
    raw_rounds: f64,
    #[serde(default, deserialize_with = "present_number")]
    transfer_acceptance: Option<f64>,
    #[serde(default, deserialize_with = "present_number")]
    transfer_error: Option<f64>,
    #[serde(default, deserialize_with = "present_number")]
    transfer_qubits: Option<f64>,
    #[serde(default, deserialize_with = "present_number")]
    transfer_rounds: Option<f64>,
}
fn present_number<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<f64>, D::Error> {
    f64::deserialize(d).map(Some)
}
fn default_error() -> f64 {
    0.01
}
fn default_shots() -> u64 {
    100_000
}
fn default_seed() -> String {
    "42".into()
}
fn default_rounds() -> usize {
    1
}
fn block_qubits() -> f64 {
    15.0
}
fn block_rounds() -> f64 {
    10.0
}
fn one() -> f64 {
    1.0
}
fn fail(e: impl std::fmt::Display) -> String {
    e.to_string()
}
fn result_json(d: DistillationResult) -> Value {
    json!({"input_error":d.input_error,"acceptance_probability":d.acceptance_probability,"rejected_probability":d.rejected_probability,"accepted_logical_error_probability":d.accepted_logical_error_probability,"conditional_output_error":d.conditional_output_error})
}
fn seed(s: &str) -> Result<u64, String> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return Err("seed must be a decimal unsigned 64 bit integer".into());
    }
    s.parse()
        .map_err(|_| "seed exceeds unsigned 64 bit range".into())
}
fn sampled(r: &FactoryRequest) -> Result<Value, String> {
    if r.shots == 0 || r.shots > 5_000_000 {
        return Err("shots must be an integer in [1,5000000]".into());
    }
    let c = sample_15_to_1(r.input_error, r.shots, seed(&r.seed)?).map_err(fail)?;
    let (a, e) = c.confidence_bounds(0.05).map_err(fail)?;
    Ok(
        json!({"attempts":c.attempts,"accepted":c.accepted,"rejected":c.rejected,"logical_errors":c.logical_errors,"seed":c.seed.to_string(),"acceptance_estimate":c.acceptance_estimate(),"conditional_output_error_estimate":c.output_error_estimate(),"confidence":{"method":"Hoeffding with union bound for two intervals","joint_coverage":0.95,"acceptance":[a.lower,a.upper],"conditional_output_error":[e.lower,e.upper]}}),
    )
}
fn resources(r: &FactoryRequest) -> Result<Value, String> {
    if r.rounds == 0 || r.rounds > 16 {
        return Err("rounds must be an integer in [1,16]".into());
    }
    let block = AttemptCost {
        qubits: r.block_qubits,
        rounds: r.block_rounds,
    };
    let raw = AttemptCost {
        qubits: r.raw_qubits,
        rounds: r.raw_rounds,
    };
    let mut error = r.input_error;
    let mut stages = Vec::new();
    let mut profiles = Vec::new();
    for _ in 0..r.rounds {
        let d = distill_15_to_1(error).map_err(fail)?;
        error = d.conditional_output_error;
        profiles.push(FactoryStage {
            input_states: 15,
            acceptance_probability: d.acceptance_probability,
            conditional_output_error_upper_bound: error,
            attempt_cost: block,
        });
        stages.push(result_json(d));
    }
    let mut estimate = estimate_factory(&profiles, raw).map_err(fail)?;
    let applied = r.transfer_acceptance.is_some()
        || r.transfer_error.is_some()
        || r.transfer_qubits.is_some()
        || r.transfer_rounds.is_some();
    let transfer = if applied {
        let a = r.transfer_acceptance.unwrap_or(1.0);
        let e = r.transfer_error.unwrap_or(0.0);
        let cost = AttemptCost {
            qubits: r.transfer_qubits.unwrap_or(0.0),
            rounds: r.transfer_rounds.unwrap_or(0.0),
        };
        estimate = apply_transfer(estimate, a, e, cost).map_err(fail)?;
        json!({"acceptance":a,"new_conditional_error_bound":e,"qubits":cost.qubits,"rounds":cost.rounds})
    } else {
        Value::Null
    };
    Ok(
        json!({"cost_basis":"illustrative defaults unless caller supplied; expected volume, not peak qubits or wall clock","block_cost":{"qubits":block.qubits,"rounds":block.rounds},"raw_cost":{"qubits":raw.qubits,"rounds":raw.rounds},"stages":stages,"transfer":transfer,"expected_raw_states":estimate.expected_raw_states,"expected_stage_attempts":estimate.expected_stage_attempts,"expected_qubit_rounds":estimate.expected_qubit_rounds,"output_error_upper_bound":estimate.output_error_upper_bound}),
    )
}

/// Execute one strictly typed request. Input size and each computational budget
/// are capped before executing numerical work. No partial numerical answer is
/// returned when an exact operation exhausts its budget.
pub fn run_json(request: &str) -> Result<String, String> {
    if request.len() > MAX_REQUEST_BYTES {
        return Err("research request exceeds 1 MiB".into());
    }
    let h: Header = serde_json::from_str(request).map_err(fail)?;
    let value = match h.op.as_str() {
        "magic-factory" | "magic-exact" | "magic-sample" | "magic-resources" => {
            let r: FactoryRequest = serde_json::from_str(request).map_err(fail)?;
            match r.op.as_str() {
                "magic-exact" => result_json(distill_15_to_1(r.input_error).map_err(fail)?),
                "magic-sample" => sampled(&r)?,
                "magic-resources" => resources(&r)?,
                _ => {
                    // Reject invalid costs before starting a potentially costly sampler.
                    let costs = resources(&r)?;
                    let counts = sampled(&r)?;
                    json!({"schema_version":1,"model":"ideal_15_to_1_independent_Z","hardware_run":false,"physical_cultivation_reproduction":false,"assumptions":["independent input Z faults","ideal Clifford operations and syndrome checks","independent outputs between stages","no noise or latency inferred for hardware"],"exact":result_json(distill_15_to_1(r.input_error).map_err(fail)?),"sample":counts,"resources":costs})
                }
            }
        }
        "magic-classify" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct R {
                op: String,
                mask: u16,
            }
            let r: R = serde_json::from_str(request).map_err(fail)?;
            let c = classify_faults(r.mask).map_err(fail)?;
            json!({"accepted":c.accepted(),"logical_error":c.logical_error})
        }
        "magic-spectrum" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct R {
                op: String,
            }
            let _: R = serde_json::from_str(request).map_err(fail)?;
            let (a, b) = weight_enumerators();
            json!((0..16).map(|i| a[i] + b[i]).collect::<Vec<_>>())
        }
        "cultivation" => cultivate(request)?,
        "hypergraph-decode" => decode(request)?,
        "pauli-evaluate" => propagate(request)?,
        _ => return Err(format!("unsupported research operation: {}", h.op)),
    };
    serde_json::to_string(&value).map_err(fail)
}

fn decode(request: &str) -> Result<Value, String> {
    use crate::hypergraph_decoder::{ErrorMechanism, HypergraphDecoder};
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Mechanism {
        detectors: String,
        logicals: String,
        probability: f64,
    }
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct R {
        op: String,
        detector_count: u32,
        logical_count: u32,
        mechanisms: Vec<Mechanism>,
        syndrome: String,
        max_nodes: u64,
    }
    fn mask(s: &str) -> Result<u128, String> {
        if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
            return Err("masks must be unsigned decimal strings".into());
        }
        s.parse()
            .map_err(|_| "mask exceeds unsigned 128 bit range".into())
    }
    let r: R = serde_json::from_str(request).map_err(fail)?;
    if r.max_nodes == 0 || r.max_nodes > 1_000_000 {
        return Err("max_nodes must be in [1,1000000]".into());
    }
    if r.mechanisms.len() > 64 {
        return Err("at most 64 mechanisms".into());
    }
    let mechanisms = r
        .mechanisms
        .iter()
        .map(|m| {
            Ok(ErrorMechanism {
                detectors: mask(&m.detectors)?,
                logicals: mask(&m.logicals)?,
                probability: m.probability,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let d = HypergraphDecoder::new(r.detector_count, r.logical_count, mechanisms)
        .map_err(fail)?
        .decode(mask(&r.syndrome)?, r.max_nodes)
        .map_err(fail)?;
    Ok(
        json!({"schema_version":1,"model":"independent_bernoulli_fault_hypergraph","selected_mechanisms":d.selected_mechanisms.to_string(),"logical_mask":d.logical_mask.to_string(),"log_odds_cost":d.log_odds_cost,"visited_nodes":d.visited_nodes,"hardware_run":false}),
    )
}
fn propagate(request: &str) -> Result<Value, String> {
    use crate::{
        gate::Gate,
        pauli_propagation::{self, Budget, InputState, Operation, Pauli},
    };
    fn present_target<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<u32>, D::Error> {
        u32::deserialize(d).map(Some)
    }
    #[derive(Deserialize)]
    #[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
    enum Op {
        Gate {
            name: String,
            qubit: u32,
            #[serde(default, deserialize_with = "present_target")]
            target: Option<u32>,
        },
        PauliFault {
            pauli: String,
            probability: f64,
        },
        Postselect {
            pauli: String,
            positive: bool,
        },
    }
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct R {
        op: String,
        inputs: Vec<String>,
        operations: Vec<Op>,
        observable: String,
        max_terms: usize,
        max_work: usize,
    }
    let r: R = serde_json::from_str(request).map_err(fail)?;
    if r.inputs.len() > 256
        || r.operations.len() > 4096
        || r.max_terms == 0
        || r.max_terms > 8192
        || r.max_work == 0
        || r.max_work > 32_000_000
    {
        return Err(
            "Pauli JSON budget exceeded (256 qubits,4096 operations,8192 terms,32000000 work)"
                .into(),
        );
    }
    let inputs = r
        .inputs
        .iter()
        .map(|s| match s.as_str() {
            "zero" => Ok(InputState::Zero),
            "plus" => Ok(InputState::Plus),
            "t" => Ok(InputState::T),
            _ => Err("input must be zero, plus or t".to_string()),
        })
        .collect::<Result<Vec<_>, _>>()?;
    let operations = r
        .operations
        .into_iter()
        .map(|o| {
            Ok(match o {
                Op::PauliFault { pauli, probability } => Operation::PauliFault {
                    pauli: Pauli::parse(&pauli).map_err(fail)?,
                    probability,
                },
                Op::Postselect { pauli, positive } => Operation::Postselect {
                    pauli: Pauli::parse(&pauli).map_err(fail)?,
                    positive,
                },
                Op::Gate {
                    name,
                    qubit,
                    target,
                } => {
                    if name != "cnot" && target.is_some() {
                        return Err("target is valid only for cnot".into());
                    }
                    Operation::Gate(match name.as_str() {
                        "h" => Gate::H(qubit),
                        "s" => Gate::S(qubit),
                        "sdg" => Gate::Sdg(qubit),
                        "x" => Gate::X(qubit),
                        "y" => Gate::Y(qubit),
                        "z" => Gate::Z(qubit),
                        "t" => Gate::T(qubit),
                        "tdg" => Gate::Tdg(qubit),
                        "cnot" => Gate::CNOT(qubit, target.ok_or("cnot requires target")?),
                        _ => return Err("unsupported Pauli gate".into()),
                    })
                }
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let p = pauli_propagation::evaluate(
        &inputs,
        &operations,
        &Pauli::parse(&r.observable).map_err(fail)?,
        Budget {
            max_terms: r.max_terms,
            max_work: r.max_work,
        },
    )
    .map_err(fail)?;
    Ok(
        json!({"schema_version":1,"model":"bounded_sparse_pauli_propagation","acceptance_probability":p.acceptance_probability,"weighted_expectation":p.weighted_expectation,"conditional_expectation":p.conditional_expectation,"peak_terms":p.peak_terms,"work":p.work,"hardware_run":false}),
    )
}

fn cultivate(request: &str) -> Result<Value, String> {
    use crate::cultivation::{run_surface_cultivation, CultivationConfig};
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct R {
        op: String,
        #[serde(default = "cultivation_attempts")]
        attempts: u32,
        #[serde(default = "default_seed")]
        seed: String,
        #[serde(default)]
        gate_error: f64,
        #[serde(default)]
        preparation_error: f64,
        #[serde(default)]
        measurement_error: f64,
        #[serde(default = "cultivation_rounds")]
        verification_rounds: u32,
    }
    fn cultivation_attempts() -> u32 {
        16
    }
    fn cultivation_rounds() -> u32 {
        2
    }
    let r: R = serde_json::from_str(request).map_err(fail)?;
    // Dense 16-qubit trajectories are deliberately bounded much more tightly
    // than the inexpensive independent-fault sampler.
    if r.attempts == 0 || r.attempts > 256 {
        return Err("cultivation attempts must be in [1,256]".into());
    }
    if r.verification_rounds != 2 {
        return Err(
            "published cultivation protocol requires exactly two verification rounds".into(),
        );
    }
    let initial_seed = seed(&r.seed)?;
    let config = CultivationConfig {
        attempts: r.attempts,
        seed: initial_seed,
        gate_error: r.gate_error,
        preparation_error: r.preparation_error,
        measurement_error: r.measurement_error,
        verification_rounds: r.verification_rounds,
    };
    let c = run_surface_cultivation(&config)?;
    Ok(
        json!({"schema_version":1,"model":"circuit_level_surface_cultivation_f3","hardware_run":false,"protocol_id":c.protocol_id,"scope":"published f3 cultivation stage; escape and final decoder excluded","logical_h_label":"joint codespace and logical HXY minus probability","seed":initial_seed.to_string(),"noise":{"gate_error":r.gate_error,"preparation_error":r.preparation_error,"measurement_error":r.measurement_error},"attempts":c.attempts,"accepted":c.accepted,"rejected":c.rejected,"rejected_by_stage":c.rejected_by_stage,"acceptance_probability":c.acceptance_probability,"acceptance_wilson_95":c.acceptance_wilson_95,"sampled_output_error_events":c.sampled_output_error_events,"output_error_wilson_95":c.output_error_wilson_95,"mean_accepted_fidelity":c.mean_accepted_fidelity,"mean_codespace_probability":c.mean_codespace_probability,"mean_logical_h_minus_probability":c.mean_logical_h_minus_probability,"physical_gates":c.physical_gates,"measurements":c.measurements,"preparations":c.preparations,"sampled_fault_locations":c.sampled_fault_locations,"executed_qubit_operations":c.executed_qubit_operations,"peak_state_bytes":c.peak_state_bytes}),
    )
}
