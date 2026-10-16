use ruqu_core::research_api::run_json;
use serde_json::{json, Value};
fn run(v: Value) -> Value {
    serde_json::from_str(&run_json(&v.to_string()).unwrap()).unwrap()
}
#[test]
fn shared_sampler_preserves_cli_golden_counts_and_large_seed() {
    let r = run(json!({"op":"magic-factory","shots":100000,"seed":"42"}));
    assert_eq!(r["sample"]["accepted"], 86063);
    assert_eq!(r["sample"]["rejected"], 13937);
    assert_eq!(r["sample"]["logical_errors"], 3);
    assert_eq!(r["resources"]["transfer"], Value::Null);
    let s = run(json!({"op":"magic-sample","shots":1,"seed":"18446744073709551615"}));
    assert_eq!(s["seed"], "18446744073709551615");
}
#[test]
fn malformed_requests_and_budget_bypasses_are_rejected() {
    for v in [
        json!({"op":"magic-factory","seed":42}),
        json!({"op":"magic-factory","shots":true}),
        json!({"op":"magic-factory","shots":0}),
        json!({"op":"magic-factory","shots":5000001}),
        json!({"op":"magic-factory","rounds":17}),
        json!({"op":"magic-factory","seed":"18446744073709551616"}),
        json!({"op":"magic-factory","extra":0}),
        json!({"op":"magic-factory","input_error":1e-200}),
        json!({"op":"magic-factory","transfer_acceptance":0}),
        json!({"op":"magic-factory","block_qubits":1e308,"block_rounds":1e308}),
    ] {
        assert!(run_json(&v.to_string()).is_err(), "{v}");
    }
    assert!(run_json(r#"{"op":"magic-sample","shots":1,"shots":2}"#).is_err());
    assert!(run_json(&" ".repeat(1_048_577)).is_err());
}
#[test]
fn transfer_accounts_for_rejected_preparations() {
    let a = run(json!({"op":"magic-resources","rounds":2}));
    let b = run(
        json!({"op":"magic-resources","rounds":2,"transfer_acceptance":0.5,"transfer_error":0.00001,"transfer_qubits":2,"transfer_rounds":5}),
    );
    assert_eq!(
        b["expected_raw_states"].as_f64().unwrap(),
        2.0 * a["expected_raw_states"].as_f64().unwrap()
    );
    assert_eq!(
        b["expected_stage_attempts"].as_f64().unwrap(),
        2.0 * (a["expected_stage_attempts"].as_f64().unwrap() + 1.0)
    );
    assert_eq!(
        b["expected_qubit_rounds"].as_f64().unwrap(),
        2.0 * (a["expected_qubit_rounds"].as_f64().unwrap() + 10.0)
    );
}
#[test]
fn bell_pauli_and_correlated_decoder_are_available() {
    let p = run(
        json!({"op":"pauli-evaluate","inputs":["zero","zero"],"operations":[{"kind":"gate","name":"h","qubit":0},{"kind":"gate","name":"cnot","qubit":0,"target":1}],"observable":"XX","max_terms":64,"max_work":10000}),
    );
    assert!((p["conditional_expectation"].as_f64().unwrap() - 1.0).abs() < 1e-12);
    let d = run(
        json!({"op":"hypergraph-decode","detector_count":2,"logical_count":1,"mechanisms":[{"detectors":"3","logicals":"1","probability":0.1}],"syndrome":"3","max_nodes":100}),
    );
    assert_eq!(d["selected_mechanisms"], "1");
    assert_eq!(d["logical_mask"], "1");
}

#[test]
fn explicit_null_booleans_and_budget_exhaustion_never_produce_partial_answers() {
    assert!(run_json(r#"{"op":"magic-factory","transfer_acceptance":null}"#).is_err());
    assert!(run_json(r#"{"op":"pauli-evaluate","inputs":["zero"],"operations":[{"kind":"postselect","pauli":"Z","positive":1}],"observable":"Z","max_terms":4,"max_work":100}"#).is_err());
    assert!(run_json(r#"{"op":"pauli-evaluate","inputs":["zero"],"operations":[],"observable":"Z","max_terms":4,"max_work":1}"#).is_err());
    assert!(run_json(r#"{"op":"hypergraph-decode","detector_count":2,"logical_count":1,"mechanisms":[{"detectors":"3","logicals":"1","probability":0.1}],"syndrome":"3","max_nodes":1}"#).is_err());
}

#[test]
fn cultivation_input_work_budget_is_strict() {
    for v in [
        json!({"op":"cultivation","attempts":0}),
        json!({"op":"cultivation","attempts":257}),
        json!({"op":"cultivation","verification_rounds":1}),
        json!({"op":"cultivation","seed":42}),
        json!({"op":"cultivation","attempts":1,"gate_error":true}),
        json!({"op":"cultivation","attempts":1,"extra":0}),
    ] {
        assert!(run_json(&v.to_string()).is_err(), "{v}");
    }
}

#[test]
fn published_cultivation_routes_and_reports_every_attempt() {
    let r = run(json!({"op":"cultivation","attempts":1,"seed":"42"}));
    assert_eq!(r["attempts"], 1);
    assert_eq!(
        r["accepted"].as_u64().unwrap() + r["rejected"].as_u64().unwrap(),
        1
    );
    assert_eq!(r["hardware_run"], false);
    assert!(r["scope"].as_str().unwrap().contains("escape"));
    assert!(r["physical_gates"].as_u64().unwrap() > 0);
    assert!(r["output_error_wilson_95"].is_null() || r["output_error_wilson_95"].is_array());
}
