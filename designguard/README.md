# DesignGuard

[![Crates.io](https://img.shields.io/crates/v/designguard.svg)](https://crates.io/crates/designguard)
[![Documentation](https://docs.rs/designguard/badge.svg)](https://docs.rs/designguard)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

KiCad schematic and PCB validation library for Rust.

Catches common design mistakes before manufacturing:

- Missing decoupling capacitors
- Incorrect pull-up/pull-down resistors
- Trace width violations (IPC-2221)
- Component datasheet compliance
- Power integrity issues
- EMI/EMC concerns

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
designguard = "0.1"
```

## Quick Start

```rust
use designguard::prelude::*;
use std::path::Path;

fn main() -> Result<(), DesignGuardError> {
    let options = ValidationOptions::default();

    let result = DesignGuardCore::validate_schematic(
        Path::new("design.kicad_sch"),
        options,
    )?;

    for issue in &result.issues {
        println!("{:?}: {}", issue.severity, issue.message);
    }

    if result.has_critical() {
        eprintln!("Critical issues found!");
        std::process::exit(1);
    }

    Ok(())
}
```

## Features

### Schematic validation

- **Decoupling capacitors**: 100nF bypass caps near IC power pins
- **I2C pull-ups**: 2.2k–10k on SDA/SCL nets
- **Crystal load caps**: 10–33pF paired capacitors
- **Power integrity**: GND symbols and IC power connections
- **ESD protection**: TVS diodes on USB/Ethernet
- **Datasheet checks**: Built-in component library (STM32, ESP32, RP2040, etc.)

### PCB validation

- **Trace width**: IPC-2221 current capacity
- **EMI checks**: Plane gaps, via placement, crosstalk
- **Custom rules**: JSON rule engine

## Examples

See the [`examples/`](examples/) directory:

- [`simple_validation.rs`](examples/simple_validation.rs) – basic usage
- [`custom_rules.rs`](examples/custom_rules.rs) – using `RulesEngine` with default rules

## CLI and GUI

- **CLI**: [`designguard-cli`](https://github.com/ltwmori/designGuardDesktopApp) (same repo, workspace member)
- **GUI**: [DesignGuard desktop app](https://design-guard-git-main-ltwmoris-projects.vercel.app/)

## Documentation

- **API docs (when published):** [docs.rs/designguard](https://docs.rs/designguard). If the crate was just published, docs.rs may take a few minutes to build; you can [request a rebuild](https://docs.rs/crate/designguard) if needed.
- **Build docs locally:** From the repo root run `cargo doc --no-deps -p designguard --open` to build and open the API docs (same as [docs.rs](https://docs.rs/about/builds)).

## License

MIT – see [LICENSE](../LICENSE) in the repository root.
