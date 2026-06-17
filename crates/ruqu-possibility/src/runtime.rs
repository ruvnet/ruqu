//! The [`PossibilityRuntime`] trait: the common contract for any system that
//! constructs a possibility field, scores it via interference, gates it, and
//! collapses it to a single output with a receipt.
//!
//! See ADR-258 §16.1.

use crate::field::PossibilityField;
use crate::gate::GateDecision;
use crate::receipt::CollapseReceipt;

/// A four-stage possibility pipeline: construct → interfere → gate → collapse.
///
/// `I` is the runtime input (a query, an event, a set of candidates); `O` is the
/// payload type carried by each possibility (a retrieval chunk, a plan, ...).
pub trait PossibilityRuntime<I, O> {
    /// The error type produced by the runtime.
    type Error;

    /// Build a possibility field from raw input.
    fn construct_field(&self, input: I) -> std::result::Result<PossibilityField<O>, Self::Error>;

    /// Apply interference scoring, amplifying coherent candidates and
    /// suppressing contradictory ones.
    fn interfere(
        &self,
        field: PossibilityField<O>,
    ) -> std::result::Result<PossibilityField<O>, Self::Error>;

    /// Convert the field's structural risk into a gate decision.
    fn gate(&self, field: &PossibilityField<O>) -> std::result::Result<GateDecision, Self::Error>;

    /// Collapse the field to a single output and emit a receipt.
    fn collapse(
        &self,
        field: PossibilityField<O>,
    ) -> std::result::Result<(O, CollapseReceipt), Self::Error>;
}
