//! Deterministic, offline projection of independently anchored synthetic records.
//! A digest proves consistency with the supplied root, not physical truth.

use rufield_core::{
    Destination, FieldAxis, FieldEvent, FieldTensor, Modality, Observation, PrivacyClass,
    PrivacyDecision, ProvenanceRef, SensorDescriptor,
};
use rufield_privacy::DefaultPrivacyGuard;
use rufield_provenance::TrustVerifier;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt;
use std::io::Read;
use wifi_densepose_geo::GeoRegistration;
use wifi_densepose_worldgraph::{
    SemanticProvenance, WorldEdge, WorldGraph, WorldGraphSnapshot, WorldId, WorldNode,
    ZoneBoundsEnu,
};

pub const MAX_INPUT_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_EVENTS: usize = 10_000;
pub const PROTOCOL: &str = "ruqu.observer.v1";
pub const BACKEND: &str = "classical state-vector simulation; no QPU";
pub const RUFIELD_REVISION: &str = "7179a2efc706993ee0d87f0093e6a0e9e3dc5017";
pub const WORLDGRAPH_REVISION: &str = "9b1c79c836cdacfb7b44f058c593157bac4c1dab";

#[derive(Debug)]
pub struct IntegrationError(String);

impl fmt::Display for IntegrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for IntegrationError {}

type Result<T> = std::result::Result<T, IntegrationError>;

fn failure(message: &str) -> IntegrationError {
    IntegrationError(message.to_owned())
}

fn require(condition: bool, message: &str) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(failure(message))
    }
}

/// Only these report fields are consumed. Other analytic report sections are
/// deliberately ignored and never become graph facts. Nested evidence is strict.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Bundle {
    schema: u32,
    protocol: String,
    backend: String,
    wasm_sha256: String,
    ledger: Ledger,
    memories: Vec<Memory>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Ledger {
    rows: Vec<Row>,
    root: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Row {
    previous: String,
    record: Record,
    hash: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Record {
    sequence: u32,
    source: String,
    seed: u32,
    setting: [u8; 2],
    outcome: u8,
    alice: u8,
    bob: u8,
    engine_sha256: String,
    protocol: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Memory {
    sequence: u32,
    alice_recall: u8,
    bob_recall: u8,
    intervention: String,
}

/// Validated input cannot be constructed by callers without verification.
pub struct VerifiedBundle {
    bundle: Bundle,
    trusted_root: String,
}

/// Output uses the real pinned upstream types, not lookalike JSON structs.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Projection {
    pub schema: String,
    pub protocol: String,
    pub synthetic: bool,
    pub trusted_root: String,
    pub verification: String,
    pub geographic_registration: String,
    pub time_basis: String,
    pub rufield_revision: String,
    pub worldgraph_revision: String,
    pub event_count: usize,
    pub disagreement_count: usize,
    pub field_events: Vec<FieldEvent>,
    pub worldgraph: WorldGraphSnapshot,
}

fn is_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Explicit recursive ordering avoids coupling the wire contract to serde_json
/// feature unification or insertion order. This protocol uses only integers and
/// fixed ASCII strings, avoiding cross-language floating-point JSON ambiguity.
fn canonical(value: &serde_json::Value) -> Result<Vec<u8>> {
    fn write(value: &serde_json::Value, out: &mut Vec<u8>) -> Result<()> {
        match value {
            serde_json::Value::Object(object) => {
                out.push(b'{');
                let ordered: BTreeMap<_, _> = object.iter().collect();
                for (index, (key, value)) in ordered.into_iter().enumerate() {
                    if index > 0 {
                        out.push(b',');
                    }
                    serde_json::to_writer(&mut *out, key)
                        .map_err(|_| failure("canonical key encoding failed"))?;
                    out.push(b':');
                    write(value, out)?;
                }
                out.push(b'}');
            }
            serde_json::Value::Array(values) => {
                out.push(b'[');
                for (index, value) in values.iter().enumerate() {
                    if index > 0 {
                        out.push(b',');
                    }
                    write(value, out)?;
                }
                out.push(b']');
            }
            _ => serde_json::to_writer(&mut *out, value)
                .map_err(|_| failure("canonical value encoding failed"))?,
        }
        Ok(())
    }
    let mut out = Vec::new();
    write(value, &mut out)?;
    Ok(out)
}

fn record_digest(record: &Record) -> Result<String> {
    let value = serde_json::to_value(record).map_err(|_| failure("record encoding failed"))?;
    Ok(digest(&canonical(&value)?))
}

fn row_digest(previous: &str, record: &Record) -> Result<String> {
    Ok(digest(&canonical(
        &serde_json::json!({"previous": previous, "record": record}),
    )?))
}

/// Read no more than one bounded input document. No file paths or URLs are
/// accepted by the library; the caller supplies an already authorized stream.
pub fn read_bounded(reader: impl Read) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader
        .take((MAX_INPUT_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| failure("failed reading input"))?;
    require(bytes.len() <= MAX_INPUT_BYTES, "input exceeds 32 MiB limit")?;
    Ok(bytes)
}

/// Verify the entire evidence and recall bundle before allocating any graph.
/// The root MUST arrive separately from trusted experiment configuration.
pub fn verify_input(bytes: &[u8], trusted_root: &str) -> Result<VerifiedBundle> {
    require(bytes.len() <= MAX_INPUT_BYTES, "input exceeds 32 MiB limit")?;
    require(
        is_digest(trusted_root),
        "trusted root must be 64 lowercase hexadecimal characters",
    )?;
    let bundle: Bundle = serde_json::from_slice(bytes)
        .map_err(|_| failure("invalid observer bundle JSON or schema"))?;
    require(bundle.schema == 1, "unsupported bundle schema")?;
    require(bundle.protocol == PROTOCOL, "unsupported envelope protocol")?;
    require(
        bundle.backend == BACKEND,
        "only the declared classical simulator backend is accepted",
    )?;
    require(is_digest(&bundle.wasm_sha256), "invalid engine digest")?;
    let rows = &bundle.ledger.rows;
    require(
        !rows.is_empty() && rows.len() <= MAX_EVENTS,
        "event count must be 1..=10000",
    )?;
    require(
        bundle.memories.len() == rows.len(),
        "memory count differs from evidence count",
    )?;
    require(
        bundle.ledger.root == trusted_root,
        "bundle root differs from external trusted root",
    )?;
    let seed = rows[0].record.seed;
    require(seed != 0, "seed must be nonzero")?;
    let mut previous = "0".repeat(64);
    for (index, (row, memory)) in rows.iter().zip(&bundle.memories).enumerate() {
        let record = &row.record;
        require(
            record.sequence as usize == index && memory.sequence as usize == index,
            "record and memory sequences must be contiguous and aligned",
        )?;
        require(
            record.source == "ruqu-wasm-simulation" && record.protocol == PROTOCOL,
            "unsupported source or protocol",
        )?;
        require(
            record.engine_sha256 == bundle.wasm_sha256,
            "record engine digest differs from report",
        )?;
        require(record.seed == seed, "seed must be constant across one run")?;
        require(
            record.setting == [0, 0],
            "protocol v1 accepts only zero-angle ledger settings",
        )?;
        require(
            matches!(record.outcome, 0 | 3),
            "zero-angle Bell outcome must be 0 or 3",
        )?;
        require(
            record.alice == (record.outcome & 1) && record.bob == ((record.outcome >> 1) & 1),
            "outcome and observer bits disagree",
        )?;
        require(
            is_digest(&row.hash) && row.previous == previous,
            "invalid row digest or chain predecessor",
        )?;
        require(
            row_digest(&previous, record)? == row.hash,
            "row digest verification failed",
        )?;
        let intervened = index % 4 == 0;
        require(
            memory.alice_recall == record.alice
                && memory.bob_recall == record.bob ^ u8::from(intervened)
                && memory.intervention
                    == if intervened {
                        "deliberate-bit-flip"
                    } else {
                        "none"
                    },
            "memory differs from the fixed declared intervention policy",
        )?;
        previous.clone_from(&row.hash);
    }
    require(
        previous == trusted_root,
        "ledger does not terminate at external trusted root",
    )?;
    Ok(VerifiedBundle {
        bundle,
        trusted_root: trusted_root.to_owned(),
    })
}

fn field_event(row: &Row, trusted_root: &str) -> Result<FieldEvent> {
    let record = &row.record;
    // A deterministic fixture time, not a claim about real acquisition time.
    // Positive and < 2^53 across the event cap, so downstream JSON consumers
    // can preserve exact nanoseconds without a BigInt-aware JSON parser.
    let timestamp_ns = (u64::from(record.sequence) + 1) * 1_000_000;
    let tensor = FieldTensor::new(
        timestamp_ns,
        Modality::SyntheticSim,
        vec![FieldAxis::Channel],
        vec![2],
        vec![f32::from(record.alice), f32::from(record.bob)],
        1.0,
        0.0,
        None,
        PrivacyClass::P2,
    )
    .map_err(|_| failure("invalid field tensor"))?;
    let observation = Observation {
        zone_id: Some("synthetic-observer-lab-unregistered".into()),
        space_cell: None,
        range_m: None,
        velocity_mps: None,
        motion_vector: None,
        track_id: None,
        confidence: 1.0,
        features: BTreeMap::from([
            ("alice_bit".into(), f32::from(record.alice)),
            ("bob_bit".into(), f32::from(record.bob)),
        ]),
        attributes: BTreeMap::from([
            ("protocol".into(), PROTOCOL.into()),
            ("evidence_row_hash".into(), row.hash.clone()),
            ("trusted_ledger_root".into(), trusted_root.into()),
            (
                "synthetic_time_basis".into(),
                "fixture_epoch_plus_sequence_ms".into(),
            ),
            (
                "confidence_meaning".into(),
                "deterministic_projection_not_physical_accuracy".into(),
            ),
        ]),
        labels: vec!["synthetic_sim".into(), "observer_lab".into()],
        privacy_class: PrivacyClass::P2,
        identity_evidence: None,
        channel_sounding_provenance: None,
    };
    let event = FieldEvent::new(
        format!("ruqu-observer:{}:{}", trusted_root, record.sequence),
        timestamp_ns,
        SensorDescriptor {
            modality: "synthetic_sim".into(),
            vendor: "ruqu".into(),
            device_id: format!("ruqu-observer-lab:{trusted_root}"),
            placement: "unregistered synthetic fixture, not a physical sensor".into(),
            coordinate_frame: None,
            position_m: None,
            orientation_xyzw: None,
            clock_domain: "synthetic_fixture_epoch_ns".into(),
        },
        tensor,
        observation,
        ProvenanceRef {
            raw_hash: format!("sha256:{}", record_digest(record)?),
            firmware_hash: format!("sha256:{}", record.engine_sha256),
            model_id: PROTOCOL.into(),
            calibration_id: "synthetic-no-calibration".into(),
            synthetic: true,
            signature_hex: None,
            signer_pubkey_hex: None,
        },
    );
    event
        .validate_evidence_at(timestamp_ns)
        .map_err(|_| failure("field evidence validation failed"))?;
    // Upstream core checks tensor shape, not finiteness. Our narrow protocol
    // requires exactly two finite binary values and finite confidence.
    require(
        event
            .tensor
            .values
            .iter()
            .all(|v| v.is_finite() && (*v == 0.0 || *v == 1.0))
            && event.tensor.confidence.is_finite()
            && event.observation.confidence.is_finite(),
        "invalid scalar in synthetic projection",
    )?;
    Ok(event)
}

/// Construct only in-memory simulation state. This API has no external graph
/// mutation, credential, networking, filesystem or production ingestion path.
pub fn project(verified: VerifiedBundle) -> Result<Projection> {
    let VerifiedBundle {
        bundle,
        trusted_root,
    } = verified;
    let mut verifier = TrustVerifier::simulation();
    let privacy = DefaultPrivacyGuard::default();
    let mut fields = Vec::with_capacity(bundle.ledger.rows.len());
    for row in &bundle.ledger.rows {
        let event = field_event(row, &trusted_root)?;
        require(
            privacy.authorize_event(&event, Destination::EdgeLocal, false, false)
                == PrivacyDecision::Allow,
            "whole-event local privacy policy denied projection",
        )?;
        verifier
            .verify_and_record_at(&event, event.timestamp_ns)
            .map_err(|_| failure("simulation provenance or replay validation failed"))?;
        fields.push(event);
    }

    // All evidence and policy checks completed before the first graph mutation.
    let mut graph = WorldGraph::new(GeoRegistration::default());
    let room = graph.upsert_node(WorldNode::Room {
        id: WorldId::UNASSIGNED,
        area_id: None,
        name: "Synthetic observer lab: unregistered toy space, not geographic measurement".into(),
        bounds_enu: ZoneBoundsEnu::Rectangle {
            min_e: 0.0,
            min_n: 0.0,
            max_e: 1.0,
            max_n: 1.0,
        },
        floor: 0,
    });
    let mut disagreement_count = 0;
    for ((row, memory), field) in bundle.ledger.rows.iter().zip(&bundle.memories).zip(&fields) {
        let at_ms = (field.timestamp_ns / 1_000_000) as i64;
        let event = graph.upsert_node(WorldNode::Event {
            id: WorldId::UNASSIGNED,
            event_type: format!(
                "synthetic_observer_record:{}:{}",
                row.record.sequence, row.hash
            ),
            at_unix_ms: at_ms,
            located_in: Some(room),
        });
        graph
            .add_edge(
                event,
                room,
                WorldEdge::LocatedIn {
                    since_unix_ms: at_ms,
                },
            )
            .map_err(|_| failure("graph containment validation failed"))?;
        let provenance = SemanticProvenance {
            evidence: vec![
                format!("sha256:{}", row.hash),
                format!("ledger-root:{trusted_root}"),
            ],
            model_version: PROTOCOL.into(),
            calibration_version: "synthetic-no-calibration".into(),
            privacy_decision: "rufield:authorize_event:edge_local:P2:allow:synthetic_only".into(),
        };
        // Source IDs were just created and are local, so add_semantic_state's
        // permissive handling of unknown evidence sources cannot drop lineage.
        require(graph.node(event).is_some(), "missing graph evidence source")?;
        let alice = graph.add_semantic_state(
            format!(
                "Synthetic Alice recall, sequence {}: bit {}; intervention none",
                memory.sequence, memory.alice_recall
            ),
            1.0,
            at_ms,
            provenance.clone(),
            &[event],
        );
        let bob = graph.add_semantic_state(
            format!(
                "Synthetic Bob recall, sequence {}: bit {}; intervention {}",
                memory.sequence, memory.bob_recall, memory.intervention
            ),
            1.0,
            at_ms,
            provenance,
            &[event],
        );
        if memory.alice_recall != memory.bob_recall {
            disagreement_count += 1;
            graph
                .add_edge(
                    alice,
                    bob,
                    WorldEdge::Contradicts {
                        magnitude: 1.0,
                        flag: format!("synthetic-deliberate-bit-flip:{}", row.hash),
                    },
                )
                .map_err(|_| failure("graph contradiction validation failed"))?;
        }
    }
    Ok(Projection {
        schema: "ruqu.observer.world.v1".into(), protocol: PROTOCOL.into(), synthetic: true,
        trusted_root,
        verification: "consistent with externally supplied digest root; not signed, authenticated hardware, or physical truth".into(),
        geographic_registration: "unregistered synthetic placeholder; graph origin 0,0 is not measured geolocation".into(),
        time_basis: "zero-origin simulation clock: (sequence + 1) * 1000000 ns; not acquisition time".into(),
        rufield_revision: RUFIELD_REVISION.into(), worldgraph_revision: WORLDGRAPH_REVISION.into(),
        event_count: fields.len(), disagreement_count, field_events: fields, worldgraph: graph.snapshot(),
    })
}

/// Convenience interface retaining the verify-before-mutation boundary.
pub fn project_input(bytes: &[u8], trusted_root: &str) -> Result<Projection> {
    project(verify_input(bytes, trusted_root)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rufield_provenance::{TrustError, TrustPolicy, TrustedKeyRegistry};

    fn fixture(count: usize) -> (serde_json::Value, String) {
        let mut previous = "0".repeat(64);
        let mut rows = Vec::new();
        let mut memories = Vec::new();
        for index in 0..count {
            let bit = (index % 2) as u8;
            let record = Record {
                sequence: index as u32,
                source: "ruqu-wasm-simulation".into(),
                seed: 123,
                setting: [0, 0],
                outcome: bit * 3,
                alice: bit,
                bob: bit,
                engine_sha256: "a".repeat(64),
                protocol: PROTOCOL.into(),
            };
            let hash = row_digest(&previous, &record).unwrap();
            rows.push(Row {
                previous: previous.clone(),
                record,
                hash: hash.clone(),
            });
            previous = hash;
            memories.push(Memory {
                sequence: index as u32,
                alice_recall: bit,
                bob_recall: bit ^ u8::from(index % 4 == 0),
                intervention: if index % 4 == 0 {
                    "deliberate-bit-flip"
                } else {
                    "none"
                }
                .into(),
            });
        }
        (
            serde_json::json!({"schema":1, "protocol":PROTOCOL, "backend":BACKEND, "wasmSha256":"a".repeat(64),
            "ledger":{"rows":rows, "root":previous}, "memories":memories}),
            previous,
        )
    }

    fn encode(value: &serde_json::Value) -> Vec<u8> {
        serde_json::to_vec(value).unwrap()
    }

    #[test]
    fn real_upstream_types_preserve_lineage_and_exact_counts() {
        let (value, root) = fixture(8);
        let out = project_input(&encode(&value), &root).unwrap();
        assert_eq!(out.event_count, 8);
        assert_eq!(out.disagreement_count, 2);
        assert_eq!(out.worldgraph.nodes.len(), 25);
        assert_eq!(out.worldgraph.edges.len(), 26);
        assert!(out
            .field_events
            .iter()
            .all(|e| e.provenance.synthetic && e.tensor.modality == Modality::SyntheticSim));
        let encoded = serde_json::to_vec(&out.worldgraph).unwrap();
        let restored = WorldGraph::from_json(&encoded).unwrap();
        assert_eq!(restored.node_count(), 25);
        assert_eq!(restored.edge_count(), 26);
        for node in &out.worldgraph.nodes {
            if let WorldNode::SemanticState { provenance, .. } = node {
                assert_eq!(provenance.evidence.len(), 2);
                assert!(provenance
                    .privacy_decision
                    .contains("P2:allow:synthetic_only"));
            }
        }
    }

    #[test]
    fn production_and_captured_replay_reject_simulation() {
        let (value, root) = fixture(1);
        let out = project_input(&encode(&value), &root).unwrap();
        for policy in [TrustPolicy::production(), TrustPolicy::captured_replay()] {
            let mut verifier = TrustVerifier::new(policy, TrustedKeyRegistry::default());
            assert_eq!(
                verifier
                    .verify_and_record_at(&out.field_events[0], out.field_events[0].timestamp_ns),
                Err(TrustError::SyntheticRejected)
            );
        }
    }

    #[test]
    fn duplicate_synthetic_evidence_rejected() {
        let (value, root) = fixture(1);
        let out = project_input(&encode(&value), &root).unwrap();
        let mut verifier = TrustVerifier::simulation();
        let e = &out.field_events[0];
        verifier.verify_and_record_at(e, e.timestamp_ns).unwrap();
        assert!(matches!(
            verifier.verify_and_record_at(e, e.timestamp_ns),
            Err(TrustError::DuplicateEvent(_))
        ));
    }

    #[test]
    fn whole_event_privacy_cannot_hide_raw_or_sensitive_tensor() {
        let (value, root) = fixture(1);
        let mut event = project_input(&encode(&value), &root)
            .unwrap()
            .field_events
            .remove(0);
        let guard = DefaultPrivacyGuard::default();
        assert_eq!(
            guard.authorize_event(&event, Destination::Network, false, false),
            PrivacyDecision::Allow
        );
        event.tensor.privacy_class = PrivacyClass::P0;
        assert!(matches!(
            guard.authorize_event(&event, Destination::Network, false, false),
            PrivacyDecision::Deny(_)
        ));
        event.tensor.privacy_class = PrivacyClass::P4;
        assert!(matches!(
            guard.authorize_event(&event, Destination::EdgeLocal, false, false),
            PrivacyDecision::RequiresConsent(_)
        ));
        event.tensor.privacy_class = PrivacyClass::P5;
        assert!(matches!(
            guard.authorize_event(&event, Destination::EdgeLocal, false, false),
            PrivacyDecision::Deny(_)
        ));
    }

    #[test]
    fn tamper_truncation_reorder_and_wrong_anchor_fail_closed() {
        let (value, root) = fixture(8);
        assert!(project_input(&encode(&value), &"0".repeat(64)).is_err());
        let mut changed = value.clone();
        changed["ledger"]["rows"][0]["record"]["seed"] = 456.into();
        assert!(project_input(&encode(&changed), &root).is_err());
        let mut changed = value.clone();
        changed["ledger"]["rows"].as_array_mut().unwrap().pop();
        changed["memories"].as_array_mut().unwrap().pop();
        assert!(project_input(&encode(&changed), &root).is_err());
        let mut changed = value;
        changed["ledger"]["rows"].as_array_mut().unwrap().swap(0, 1);
        assert!(project_input(&encode(&changed), &root).is_err());
    }

    #[test]
    fn unanchored_memories_must_follow_fixed_interventions() {
        let (mut value, root) = fixture(8);
        value["memories"][0]["bobRecall"] = 0.into();
        assert!(project_input(&encode(&value), &root).is_err());
    }

    #[test]
    fn semantic_and_unknown_field_attacks_fail() {
        for (key, replacement) in [
            ("source", serde_json::json!("real-quantum-hardware")),
            ("outcome", serde_json::json!(7)),
            ("alice", serde_json::json!(2)),
            ("engineSha256", serde_json::json!("b".repeat(64))),
            ("protocol", serde_json::json!("alternate-universe")),
            ("extra", serde_json::json!("injected")),
        ] {
            let (mut value, root) = fixture(1);
            value["ledger"]["rows"][0]["record"][key] = replacement;
            assert!(
                project_input(&encode(&value), &root).is_err(),
                "accepted {key}"
            );
        }
        let (mut value, root) = fixture(1);
        value["wasmSha256"] = serde_json::json!("b".repeat(64));
        assert!(project_input(&encode(&value), &root).is_err());
    }

    #[test]
    fn canonical_order_is_recursive_and_cross_language_stable() {
        let value = serde_json::json!({"z":[{"b":2,"a":1}],"a":"test"});
        assert_eq!(
            String::from_utf8(canonical(&value).unwrap()).unwrap(),
            "{\"a\":\"test\",\"z\":[{\"a\":1,\"b\":2}]}"
        );
        assert_eq!(
            digest(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn bounded_reader_and_invalid_inputs_rejected() {
        assert!(read_bounded(std::io::repeat(b' ').take((MAX_INPUT_BYTES + 1) as u64)).is_err());
        let (empty, root) = fixture(0);
        assert!(project_input(&encode(&empty), &root).is_err());
        assert!(project_input(b"{}", "not-a-root").is_err());
        assert!(project_input(b"{", &"a".repeat(64)).is_err());
    }

    #[test]
    fn maximum_event_bound_is_enforced_before_graph_construction() {
        let (allowed, root) = fixture(MAX_EVENTS);
        assert!(verify_input(&encode(&allowed), &root).is_ok());
        let (too_many, root) = fixture(MAX_EVENTS + 1);
        assert!(verify_input(&encode(&too_many), &root).is_err());
    }

    #[test]
    fn envelope_policy_and_duplicate_keys_rejected() {
        for (key, replacement) in [
            ("protocol", serde_json::json!("invented")),
            ("backend", serde_json::json!("QPU")),
            ("schema", serde_json::json!(2)),
        ] {
            let (mut value, root) = fixture(1);
            value[key] = replacement;
            assert!(verify_input(&encode(&value), &root).is_err());
        }
        let (value, root) = fixture(1);
        let duplicate = serde_json::to_string(&value).unwrap().replacen(
            "\"seed\":123",
            "\"seed\":123,\"seed\":123",
            1,
        );
        assert!(verify_input(duplicate.as_bytes(), &root).is_err());
    }

    #[test]
    fn projection_is_deterministic_and_does_not_claim_real_geolocation() {
        let (value, root) = fixture(3);
        let first = project_input(&encode(&value), &root).unwrap();
        let second = project_input(&encode(&value), &root).unwrap();
        assert_eq!(
            serde_json::to_vec(&first).unwrap(),
            serde_json::to_vec(&second).unwrap()
        );
        assert!(first
            .geographic_registration
            .contains("not measured geolocation"));
        assert!(first
            .field_events
            .iter()
            .all(|e| e.sensor.position_m.is_none() && e.observation.range_m.is_none()));
    }
}
