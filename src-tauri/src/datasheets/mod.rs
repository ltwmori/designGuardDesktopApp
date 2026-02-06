//! Datasheet-Aware Design Checking System
//!
//! This module provides functionality to verify that a user's circuit meets
//! the requirements specified in component datasheets.
//!
//! # Architecture
//!
//! ```text
//! ┌──────────────┐    ┌──────────────┐    ┌──────────────┐
//! │   Schematic  │───▶│  Component   │───▶│  Datasheet   │
//! │    Parser    │    │  Identifier  │    │   Matcher    │
//! └──────────────┘    └──────────────┘    └──────┬───────┘
//!                                                │
//!                                                ▼
//!                                        ┌──────────────┐
//!                                        │  Datasheet   │
//!                                        │   Database   │
//!                                        └──────┬───────┘
//!                                               │
//!                     ┌─────────────────────────┼─────────────────────────┐
//!                     │                         │                         │
//!                     ▼                         ▼                         ▼
//!              ┌────────────┐           ┌────────────┐           ┌─────────┐
//!              │ Decoupling │           │  External  │           │ Ratings │
//!              │   Check    │           │ Components │           │  Check  │
//!              └────────────┘           └────────────┘           └─────────┘
//!                     │                         │                         │
//!                     └─────────────────────────┼─────────────────────────┘
//!                                               │
//!                                               ▼
//!                                       ┌──────────────┐
//!                                       │    Issue     │
//!                                       │   Reporter   │
//!                                       └──────────────┘
//! ```
//!
//! # Supported ICs
//!
//! The system includes built-in datasheet requirements for:
//!
//! 1. **STM32F411CEU6** - ARM Cortex-M4 MCU
//! 2. **ESP32-WROOM-32** - WiFi/BT Module
//! 3. **ATmega328P** - AVR MCU (Arduino)
//! 4. **RP2040** - Dual-core ARM Cortex-M0+
//! 5. **LM1117-3.3** - LDO Regulator
//! 6. **AMS1117-3.3** - LDO Regulator
//! 7. **CH340G** - USB-UART Bridge
//! 8. **CP2102** - USB-UART Bridge
//! 9. **NE555** - Timer IC
//! 10. **LM7805** - Linear Regulator
//!
//! # Usage
//!
//! ```rust,ignore
//! use crate::datasheets::checker::DatasheetChecker;
//!
//! let checker = DatasheetChecker::new();
//! let issues = checker.check(&schematic);
//!
//! for issue in issues {
//!     println!("{}: {}", issue.title, issue.what);
//!     println!("Why: {}", issue.why);
//!     println!("Fix: {}", issue.how_to_fix);
//! }
//! ```

pub mod schema;
pub mod matcher;
pub mod checker;
pub mod builtin;

// Re-exports for convenience
pub use schema::*;
pub use matcher::DatasheetMatcher;
pub use checker::{DatasheetChecker, DatasheetIssue};
