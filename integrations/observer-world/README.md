# Native observer world integration

Offline Rust adapter from the ruQu WASM observer laboratory into actual, revision-pinned RuField and WorldGraph types. This is classical quantum simulation plus a deliberately altered memory experiment. It is not quantum hardware, a physical sensor, a human experiment, a quantum advantage result, or evidence of alternate realities.

## Run

From the ruQu repository root:

```sh
cargo test --locked --manifest-path integrations/observer-world/Cargo.toml
node examples/observer-lab/lab.cjs > observer-report.json
cargo run --locked --release --manifest-path integrations/observer-world/Cargo.toml -- \
  --trusted-root YOUR_INDEPENDENTLY_RETAINED_ROOT < observer-report.json
```

Retain `run().ledger.root` through a separately trusted experiment channel when generating evidence. Reading the root out of the same potentially altered file immediately before verification proves only self-consistency. A hash root is neither a signature nor proof that the stated WASM program actually executed.

Input is capped at 32 MiB and 10,000 evidence rows. Nested evidence objects reject unknown and duplicate fields. The complete chain, externally supplied root, engine digest declaration, fixed protocol, bit consistency, sequence, seed consistency and every prescribed memory intervention are validated before constructing any graph. The upstream report's remaining analytic fields are ignored, never imported as facts. The input contains no configurable destination, consent, identity binding, network endpoint or code.

## Native mappings

`rufield-core`, `rufield-privacy` and `rufield-provenance` are pinned to `7179a2efc706993ee0d87f0093e6a0e9e3dc5017`. `wifi-densepose-worldgraph` and `wifi-densepose-geo` are pinned to `9b1c79c836cdacfb7b44f058c593157bac4c1dab`. Geo network features are disabled. Cargo.lock fixes the transitive resolution.

Each original evidence row becomes a `FieldEvent` with modality `SyntheticSim`, P2 tensor and observation, `synthetic=true`, exact digest references and no physical sensor pose or personal identity. A whole-event local privacy decision and a simulation-mode provenance/replay decision are required. Production and captured-replay trust modes reject these events, even though a local simulation accepts them. The signature fields remain absent.

WorldGraph receives one unregistered toy `Room`, one `Event` per original record, and two `SemanticState` recall records per event. The states retain exact evidence handles, model, explicit synthetic calibration marker and actual privacy decision. `LocatedIn`, `DerivedFrom` and `Contradicts` use native typed edges. Every fourth Bob recall is deliberately inverted. Contradictions are the declared intervention, not new physics. No `Sensor`, real person or quantum sensor modality is invented.

The WorldGraph library requires a numeric geographic registration; its default origin of zero latitude and longitude is a placeholder, not a real location. Output explicitly marks it unregistered. The one metre toy room is invented, not surveyed. Timestamps use a zero-origin simulation clock, `(sequence + 1) * 1000000` nanoseconds, not a capture clock. They remain exact safe integers for ordinary JSON consumers across the event limit. Confidence 1.0 describes deterministic projection under this protocol, not confidence in physical reality.

For N records, there are N RuField events, 1 + 3N graph nodes and 3N + ceil(N/4) edges. Verification and projection are linear in N with bounded input. The adapter writes only JSON to stdout and diagnostic failures to stderr; it never runs a server, opens sockets or changes an external graph.

## Acceptance checks

Tests cover pinned native serialization, WorldGraph round trip, exact lineage and counts, production and captured-replay rejection, replay rejection, whole-event privacy, malformed and oversized input, tampering, truncation, reordering, wrong trusted roots, engine mismatch, strict nested keys, prescribed interventions, canonical hashes and deterministic output. Run the command above; a failed assertion or rejected input exits nonzero.
