# ruqu-possibility — Structural Possibility Runtime

The common **possibility-field** abstraction for the ruQu Structural Possibility
Runtime ([ADR-258](../../docs/adr/ADR-258-structural-possibility-runtime.md)).

It lets an AI system hold multiple plausible states, amplify coherent evidence,
suppress incoherent paths, detect structural risk before acting, and emit
auditable receipts — the substrate underneath retrieval (`ruqu-rag`), agent
coordination, sensing, and governance.

## Primitives

| Type | Role |
|------|------|
| `Possibility<T>` | one competing hypothesis: payload + amplitude (magnitude + phase) + evidence |
| `PossibilityField<T>` | the field of candidates; computes `entropy`, `coherence`, `field_hash`, and `collapse` |
| `CoherenceGate` / `GateDecision` | maps structural risk to `PERMIT` / `DEFER` / `DENY` |
| `CollapseReceipt` | auditable record of why one path was selected over the rest |
| `EvidenceReceipt` / `VerifierResult` | provenance and independent-verifier evidence |
| `PossibilityRuntime` | the construct → interfere → gate → collapse trait |

## Structural quantities

- **entropy** — Shannon entropy (bits) of the normalized `amplitude²`
  distribution; `0` for a collapsed field, `log2(n)` for a uniform one.
- **coherence** — `|Σ aₖ e^{iφₖ}|² / (Σ aₖ)²` in `[0, 1]`; `1` when all phases
  align (constructive), `→0` when they cancel (contradictory).
- **collapse** — a seeded, deterministic weighted draw that is fully replayable
  and emits a content-addressable `CollapseReceipt`.

```rust
use ruqu_possibility::{Possibility, PossibilityField, CoherenceGate};

let field = PossibilityField::new(vec![
    Possibility::new("a", "strong, well-cited answer", 0.9, 0.0),
    Possibility::new("b", "plausible but contradicted", 0.6, std::f64::consts::PI),
]);

let decision = CoherenceGate::with_defaults().evaluate(&field);
let (selected, receipt) = field.collapse(42).unwrap();
println!("{decision:?} -> {} ({})", selected.id, receipt.receipt_hash());
```

MIT © Ruvector Team. Part of the [ruqu](../../README.md) ecosystem.
