# ruqu-receipts

Tamper-evident, replayable audit logs over the structural possibility runtime's
collapse receipts (ADR-258 §6 — governance / audit).

Every collapse in [`ruqu-possibility`](../ruqu-possibility) emits a
content-addressable `CollapseReceipt`. This crate threads those receipts onto a
BLAKE3 hash chain so the entire governance history is tamper-evident, and adds
replay verification so the runtime's determinism guarantee is checkable.

## What it provides

- **`ReceiptStore`** — an append-only log of `LoggedReceipt`s. Each appended
  entry stores `entry_hash = BLAKE3(prev_hash || receipt.receipt_hash())`,
  forming a hash chain (genesis `prev_hash` = 64 zeros).
  - `new()`, `append(receipt) -> &LoggedReceipt`, `len()`, `is_empty()`,
    `tip_hash()`.
  - `verify_chain() -> bool` recomputes every `entry_hash`, checks linkage and
    sequence ordering, and returns `false` if anything was altered, inserted,
    removed, or reordered.
  - `to_jsonl()` / `from_jsonl(&str)` for durable, append-friendly, one-object-
    per-line storage. `from_jsonl` does not implicitly trust input — follow with
    `verify_chain()`.
- **`verify_replay(receipt, field) -> ReplayVerdict`** (ADR-258 §22 Test 6) —
  re-collapses `field` with `receipt.seed` and checks the same possibility is
  selected and coherence reproduces within `1e-9`. `ReplayVerdict` reports
  `reproduced`, `selected_matches`, and `coherence_delta`.

## Example

```rust
use ruqu_possibility::{Possibility, PossibilityField};
use ruqu_receipts::{ReceiptStore, verify_replay};

let field = PossibilityField::new(vec![
    Possibility::new("a", "answer a", 0.9, 0.0),
    Possibility::new("b", "answer b", 0.4, 0.0),
]);
let (_selected, receipt) = field.collapse(7).unwrap();

let mut store = ReceiptStore::new();
store.append(receipt.clone());
assert!(store.verify_chain());

// Round-trips through JSON Lines.
let restored = ReceiptStore::from_jsonl(&store.to_jsonl()).unwrap();
assert!(restored.verify_chain());

// Replay verification against the original field.
assert!(verify_replay(&receipt, &field).reproduced);
```

## Testing

```sh
cargo test -p ruqu-receipts
```
