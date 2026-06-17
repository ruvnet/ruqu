//! # ruqu-possibility — Structural Possibility Runtime
//!
//! The common possibility-field abstraction for the ruQu Structural Possibility
//! Runtime (ADR-258). It lets an AI system *hold* multiple plausible states,
//! amplify coherent evidence, suppress incoherent paths, detect structural risk
//! before acting, and emit auditable receipts.
//!
//! ## The six primitives
//!
//! 1. **Possibility field** — [`PossibilityField`] holds competing hypotheses.
//! 2. **Interference scoring** — amplitudes (magnitude + [`Possibility::phase`])
//!    reinforce or cancel; quantified by [`PossibilityField::coherence`].
//! 3. **Coherence gate** — [`CoherenceGate`] maps structural risk to
//!    [`GateDecision::Permit`] / [`GateDecision::Defer`] / [`GateDecision::Deny`].
//! 4. **Reasoning error correction** — provided by `ruqu-exotic`'s reasoning QEC;
//!    recorded here via [`VerifierResult`]s on the [`CollapseReceipt`].
//! 5. **Reversible memory** — provided by `ruqu-exotic`; receipts make collapses
//!    replayable.
//! 6. **Collapse receipt** — [`CollapseReceipt`] records why one path was chosen.
//!
//! ## Example
//!
//! ```
//! use ruqu_possibility::{Possibility, PossibilityField, CoherenceGate};
//!
//! let field = PossibilityField::new(vec![
//!     Possibility::new("a", "strong, well-cited answer", 0.9, 0.0),
//!     Possibility::new("b", "plausible but contradicted", 0.6, std::f64::consts::PI),
//! ]);
//!
//! // Coherent, low-entropy fields permit; contradictory fields defer or deny.
//! let _decision = CoherenceGate::with_defaults().evaluate(&field);
//!
//! // Collapse is deterministic for a given seed and yields a receipt.
//! let (selected, receipt) = field.collapse(42).unwrap();
//! assert_eq!(receipt.selected_id, selected.id);
//! ```

pub mod error;
pub mod field;
pub mod gate;
pub mod receipt;
pub mod runtime;

pub use error::{PossibilityError, Result};
pub use field::{Possibility, PossibilityField};
pub use gate::{CoherenceGate, GateDecision, GateThresholds};
pub use receipt::{CollapseReceipt, EvidenceReceipt, RejectedCandidate, VerifierResult};
pub use runtime::PossibilityRuntime;

/// Crate version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
