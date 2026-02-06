//! PCB Compliance Module
//!
//! Implements IPC-2221 current calculations, EMI analysis,
//! and custom rules-based compliance checking.

pub mod ipc2221;
pub mod emi;
pub mod rules;
pub mod net_classifier;
pub mod power_net_registry;

pub use ipc2221::*;
pub use emi::*;
pub use rules::*;
pub use net_classifier::*;
