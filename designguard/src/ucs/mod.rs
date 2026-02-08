//! Unified Circuit Schema (UCS) Module
//!
//! This module provides a CAD-agnostic representation of electronic circuits.
//! It allows the application to work with circuits from different EDA tools
//! (KiCAD, Altium, EasyEDA, Eagle) using a single unified schema.
//!
//! The UCS uses petgraph for efficient graph-based circuit analysis,
//! enabling operations like:
//! - Net connectivity analysis
//! - Voltage propagation
//! - Component relationship queries
//! - Signal path tracing

pub mod schema;
pub mod circuit;
pub mod adapters;
pub mod analysis;

// Re-export main types for convenience
pub use schema::*;
pub use circuit::Circuit;
pub use adapters::{CircuitAdapter, AdapterError};
