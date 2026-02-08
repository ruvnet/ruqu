//! # ruqu-core -- Quantum Simulation Engine
//!
//! Pure Rust state-vector quantum simulator for the ruVector stack.
//! Supports up to 25 qubits, common gates, measurement, noise models,
//! and expectation value computation.
//!
//! ## Quick Start
//!
//! ```
//! use ruqu_core::prelude::*;
//!
//! // Create a Bell state |00> + |11> (unnormalised notation)
//! let mut circuit = QuantumCircuit::new(2);
//! circuit.h(0).cnot(0, 1);
//! let result = Simulator::run(&circuit).unwrap();
//! let probs = result.state.probabilities();
//! // probs ~= [0.5, 0.0, 0.0, 0.5]
//! ```

pub mod types;
pub mod error;
pub mod gate;
pub mod state;
pub mod circuit;
pub mod simulator;
pub mod optimizer;

/// Re-exports of the most commonly used items.
pub mod prelude {
    pub use crate::types::*;
    pub use crate::error::{QuantumError, Result};
    pub use crate::gate::Gate;
    pub use crate::state::QuantumState;
    pub use crate::circuit::QuantumCircuit;
    pub use crate::simulator::{SimConfig, SimulationResult, Simulator, ShotResult};
}
