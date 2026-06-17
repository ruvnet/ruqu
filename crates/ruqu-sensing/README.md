# ruqu-sensing

Convert telemetry into syndrome streams for structural anomaly detection
(ADR-258 §11).

The crate bridges raw sensor signals to QEC-style syndrome diagnosis and to the
possibility runtime, in three stages.

## Pipeline

1. **Sensing.** A bank of `SensorChannel`s thresholds a time-series into one
   `SensorSyndrome` per timestamp — a detector-bit vector (bit *i* fires when
   channel *i* exceeds its threshold) plus a `confidence` derived from signal
   margin and inter-channel agreement.

   ```rust
   use ruqu_sensing::{SensorChannel, syndromes_from_samples};

   let channels = vec![SensorChannel::new("cpu", 0.8), SensorChannel::new("mem", 0.8)];
   let samples = vec![(1, vec![0.9, 0.2]), (2, vec![0.95, 0.9])];
   let syndromes = syndromes_from_samples(&channels, &samples).unwrap();
   assert_eq!(syndromes[0].detector_bits, vec![true, false]);
   ```

2. **Diagnosis.** `SystemTopology` wraps
   [`ruqu_exotic::syndrome_diagnosis`](../ruqu-exotic): named components become
   data qubits and connections become parity-check ancillas. `diagnose(
   fault_injection_rate, num_rounds, seed)` returns a `SystemDiagnosis` with
   `fragility_scores`, `weakest_component`, `fault_propagators`, and a
   `severity` (the max fragility score).

   ```rust
   use ruqu_sensing::SystemTopology;

   let topo = SystemTopology::new(
       vec!["a".into(), "b".into(), "c".into(), "d".into()],
       vec![(0, 1), (1, 2), (2, 3)],
   );
   let diag = topo.diagnose(0.4, 30, 77).unwrap();
   ```

3. **Explanation.** `fault_field(&syndromes, &component_labels)` builds a
   `PossibilityField<String>` of candidate root causes — amplitude from
   accumulated, confidence-weighted fired-bit mass; phase from whether a
   component's failures are correlated (phase ≈ 0, constructive) or
   anti-correlated (phase ≈ π, destructive) with the global anomaly level — so
   callers can gate and collapse to a most-likely root cause with a replayable
   receipt.

## Qubit budget

Syndrome diagnosis maps `components + connections` onto qubits, capped at 25.
Keep topologies small (≈4-6 components, ≤10 connections).

## Testing

```sh
cargo test -p ruqu-sensing
```
