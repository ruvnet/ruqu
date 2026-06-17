//! # ruqu-receipts — Auditable, replayable collapse logs (ADR-258 §6)
//!
//! A tamper-evident, replayable audit log over
//! [`CollapseReceipt`](ruqu_possibility::CollapseReceipt)s produced by the
//! structural possibility runtime.
//!
//! Every collapse in the runtime emits a content-addressable
//! [`CollapseReceipt`]. This crate threads those receipts onto a BLAKE3 hash
//! chain ([`ReceiptStore`]) so that the entire governance history becomes
//! tamper-evident: any mutation to a stored receipt breaks the chain and is
//! detected by [`ReceiptStore::verify_chain`]. The store round-trips through
//! [JSON Lines](ReceiptStore::to_jsonl) for durable, append-friendly storage.
//!
//! Beyond integrity, the crate provides *replay verification*
//! ([`verify_replay`], ADR-258 §22 Test 6): given the original field and a
//! receipt, it re-collapses with the recorded seed and confirms the same
//! possibility is selected with the same coherence — the runtime's
//! determinism guarantee made checkable.
//!
//! ## Example
//!
//! ```
//! use ruqu_possibility::{Possibility, PossibilityField};
//! use ruqu_receipts::ReceiptStore;
//!
//! let field = PossibilityField::new(vec![
//!     Possibility::new("a", "answer a", 0.9, 0.0),
//!     Possibility::new("b", "answer b", 0.4, 0.0),
//! ]);
//! let (_selected, receipt) = field.collapse(7).unwrap();
//!
//! let mut store = ReceiptStore::new();
//! store.append(receipt);
//! assert!(store.verify_chain());
//!
//! // Durable, append-friendly serialization.
//! let jsonl = store.to_jsonl();
//! let restored = ReceiptStore::from_jsonl(&jsonl).unwrap();
//! assert!(restored.verify_chain());
//! ```

use anyhow::{Context, Result};
use ruqu_possibility::{CollapseReceipt, PossibilityField};
use serde::{Deserialize, Serialize};

/// The genesis previous-hash: 64 hex zeros (the BLAKE3 hex digest width).
pub const GENESIS_PREV_HASH: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";

/// A single receipt as committed to the [`ReceiptStore`] hash chain.
///
/// Each entry binds a [`CollapseReceipt`] to its position (`seq`) and to the
/// preceding entry via `prev_hash`. The `entry_hash` is
/// `BLAKE3(prev_hash || receipt.receipt_hash())`, so altering any receipt or
/// reordering entries invalidates every downstream `entry_hash`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoggedReceipt {
    /// Zero-based sequence number / position in the chain.
    pub seq: u64,
    /// The audited collapse receipt.
    pub receipt: CollapseReceipt,
    /// `entry_hash` of the previous entry (genesis = [`GENESIS_PREV_HASH`]).
    pub prev_hash: String,
    /// `BLAKE3(prev_hash || receipt.receipt_hash())` as hex.
    pub entry_hash: String,
}

impl LoggedReceipt {
    /// Compute the canonical entry hash for a receipt linked to `prev_hash`.
    ///
    /// This is the single source of truth used both when appending and when
    /// re-verifying, so the two can never diverge.
    pub fn compute_entry_hash(prev_hash: &str, receipt: &CollapseReceipt) -> String {
        let mut hasher = blake3::Hasher::new();
        hasher.update(prev_hash.as_bytes());
        hasher.update(receipt.receipt_hash().as_bytes());
        hasher.finalize().to_hex().to_string()
    }
}

/// A tamper-evident, replayable append-only log of collapse receipts.
///
/// Receipts are linked into a BLAKE3 hash chain; see [`LoggedReceipt`]. Use
/// [`ReceiptStore::append`] to record collapses, [`ReceiptStore::verify_chain`]
/// to detect tampering, and [`ReceiptStore::to_jsonl`] /
/// [`ReceiptStore::from_jsonl`] for persistence.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ReceiptStore {
    /// The committed chain entries, in append order.
    pub entries: Vec<LoggedReceipt>,
}

impl ReceiptStore {
    /// Create an empty store.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Number of entries in the chain.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the chain is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The `entry_hash` of the most recent entry, or [`GENESIS_PREV_HASH`] if
    /// the store is empty. This is what the next [`append`](Self::append) links
    /// against.
    pub fn tip_hash(&self) -> String {
        self.entries
            .last()
            .map(|e| e.entry_hash.clone())
            .unwrap_or_else(|| GENESIS_PREV_HASH.to_string())
    }

    /// Append a receipt, assigning the next sequence number and linking it to
    /// the current chain tip. Returns the newly committed [`LoggedReceipt`].
    pub fn append(&mut self, receipt: CollapseReceipt) -> &LoggedReceipt {
        let seq = self.entries.len() as u64;
        let prev_hash = self.tip_hash();
        let entry_hash = LoggedReceipt::compute_entry_hash(&prev_hash, &receipt);
        self.entries.push(LoggedReceipt {
            seq,
            receipt,
            prev_hash,
            entry_hash,
        });
        // Safe: we just pushed.
        self.entries.last().expect("entry was just appended")
    }

    /// Re-derive the whole chain and confirm integrity.
    ///
    /// Returns `false` if any entry's `entry_hash` does not match a fresh
    /// recomputation, if linkage (`prev_hash`) is broken, or if sequence
    /// numbers are out of order. A `true` result means no receipt has been
    /// altered and no entry has been inserted, removed, or reordered.
    pub fn verify_chain(&self) -> bool {
        let mut expected_prev = GENESIS_PREV_HASH.to_string();
        for (i, entry) in self.entries.iter().enumerate() {
            if entry.seq != i as u64 {
                return false;
            }
            if entry.prev_hash != expected_prev {
                return false;
            }
            let recomputed = LoggedReceipt::compute_entry_hash(&entry.prev_hash, &entry.receipt);
            if recomputed != entry.entry_hash {
                return false;
            }
            expected_prev = entry.entry_hash.clone();
        }
        true
    }

    /// Serialize the chain as JSON Lines: one [`LoggedReceipt`] JSON object per
    /// line. Suitable for append-only storage and `grep`-friendly auditing.
    pub fn to_jsonl(&self) -> String {
        let mut out = String::new();
        for entry in &self.entries {
            // LoggedReceipt is always serializable; fall back to empty on the
            // theoretically-impossible error rather than panicking.
            let line = serde_json::to_string(entry).unwrap_or_default();
            out.push_str(&line);
            out.push('\n');
        }
        out
    }

    /// Parse a store from JSON Lines produced by [`to_jsonl`](Self::to_jsonl).
    ///
    /// Blank lines are ignored. This does **not** implicitly trust the input:
    /// callers should follow with [`verify_chain`](Self::verify_chain) to
    /// confirm integrity of the restored log.
    pub fn from_jsonl(input: &str) -> Result<Self> {
        let mut entries = Vec::new();
        for (lineno, raw) in input.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() {
                continue;
            }
            let entry: LoggedReceipt = serde_json::from_str(line)
                .with_context(|| format!("failed to parse JSONL entry on line {}", lineno + 1))?;
            entries.push(entry);
        }
        Ok(Self { entries })
    }
}

/// The outcome of [`verify_replay`]: did re-collapsing the field reproduce the
/// audited decision?
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ReplayVerdict {
    /// True iff the selection matched **and** coherence reproduced within
    /// `1e-9`. The headline pass/fail for ADR-258 §22 Test 6.
    pub reproduced: bool,
    /// Whether the re-collapse selected the same possibility id.
    pub selected_matches: bool,
    /// `|replayed_coherence - receipt.coherence|`.
    pub coherence_delta: f64,
}

/// Replay-verify a receipt against the field it claims to describe
/// (ADR-258 §22 Test 6).
///
/// Re-collapses `field` with `receipt.seed` and checks that the same
/// possibility is selected and that coherence reproduces within `1e-9`. A field
/// that has been altered (different candidates, amplitudes, or phases) will fail
/// to reproduce the selection and/or the coherence.
///
/// ```
/// use ruqu_possibility::{Possibility, PossibilityField};
/// use ruqu_receipts::verify_replay;
///
/// let field = PossibilityField::new(vec![
///     Possibility::new("a", "x", 0.9, 0.0),
///     Possibility::new("b", "y", 0.3, 0.0),
/// ]);
/// let (_sel, receipt) = field.collapse(11).unwrap();
/// assert!(verify_replay(&receipt, &field).reproduced);
/// ```
pub fn verify_replay<T: Clone>(
    receipt: &CollapseReceipt,
    field: &PossibilityField<T>,
) -> ReplayVerdict {
    match field.collapse(receipt.seed) {
        Ok((_selected, replayed)) => {
            let selected_matches = replayed.selected_id == receipt.selected_id;
            let coherence_delta = (replayed.coherence - receipt.coherence).abs();
            ReplayVerdict {
                reproduced: selected_matches && coherence_delta < 1e-9,
                selected_matches,
                coherence_delta,
            }
        }
        Err(_) => ReplayVerdict {
            reproduced: false,
            selected_matches: false,
            coherence_delta: f64::INFINITY,
        },
    }
}

/// Crate version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
