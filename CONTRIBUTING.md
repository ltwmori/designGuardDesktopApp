# Contributing to DesignGuard

Thank you for your interest in contributing to DesignGuard! The fastest way to get
involved is to **add a new design rule** to the schematic analysis engine. This guide
will walk you through the process step by step.

For architecture details and algorithm documentation, see [ARCHITECTURE.md](ARCHITECTURE.md).

---

## Table of Contents

- [Quick Start](#quick-start)
- [How the Rules Engine Works](#how-the-rules-engine-works)
- [Tutorial: Create Your First Rule](#tutorial-create-your-first-rule)
- [Good First Issues](#good-first-issues)
- [Development Setup](#development-setup)
- [Code Style & Conventions](#code-style--conventions)
- [Submitting a Pull Request](#submitting-a-pull-request)

---

## Quick Start

```bash
# 1. Fork and clone the repository
git clone https://github.com/<your-username>/designguard.git
cd designguard

# 2. Install dependencies
npm install

# 3. Run the Rust tests to make sure everything passes
cd src-tauri && cargo test

# 4. Make your changes (see tutorial below)

# 5. Run tests again to verify
cargo test

# 6. Open a pull request
```

---

## How the Rules Engine Works

DesignGuard uses a **trait-based rules engine** in Rust. Every design check is a
struct that implements the `Rule` trait. The engine iterates over all registered
rules and collects issues.

### The `Rule` Trait

```rust
// src-tauri/src/analyzer/rules.rs

pub trait Rule: Send + Sync {
    fn id(&self) -> &str;               // Unique identifier, e.g. "led_series_resistor"
    fn name(&self) -> &str;             // Human-readable name
    fn severity(&self) -> Severity;     // Error, Warning, Info, or Suggestion
    fn check(&self, schematic: &Schematic) -> Vec<Issue>;  // The actual check
}
```

### The `Issue` Struct

When your rule finds a problem, it returns one or more `Issue` values:

```rust
pub struct Issue {
    pub id: String,                    // Auto-generated UUID
    pub rule_id: String,               // Matches your Rule::id()
    pub severity: Severity,            // Error | Warning | Info | Suggestion
    pub message: String,               // What went wrong
    pub component: Option<String>,     // Which component (e.g. "D1")
    pub location: Option<Position>,    // Where in the schematic
    pub suggestion: Option<String>,    // How to fix it
    pub risk_score: Option<RiskScore>, // Optional quantified risk
}
```

### The `Schematic` Data Model

Your rule receives a `Schematic` struct that contains the parsed design:

```rust
pub struct Schematic {
    pub components: Vec<Component>,     // All regular components (R, C, U, D, etc.)
    pub power_symbols: Vec<Component>,  // Power symbols (GND, VCC, etc.)
    pub wires: Vec<Wire>,              // Wire connections
    pub labels: Vec<Label>,            // Net labels (SDA, SCL, RESET, etc.)
    pub nets: Vec<Net>,                // Named nets
    // ...
}
```

Each `Component` has:

```rust
pub struct Component {
    pub reference: String,   // "R1", "C1", "U1", "D1"
    pub value: String,       // "10k", "100nF", "STM32F4", "LED_Red"
    pub lib_id: String,      // KiCad library identifier
    pub position: Position,  // X/Y coordinates in the schematic
    pub properties: HashMap<String, String>,
    pub pins: Vec<Pin>,
    // ...
}
```

### How Rules Are Registered

Rules are added to the engine in `RulesEngine::with_default_rules()`:

```rust
pub fn with_default_rules() -> Self {
    let mut engine = Self::new();
    engine.add_rule(Arc::new(DecouplingCapacitorRule));
    engine.add_rule(Arc::new(I2CPullResistorRule));
    engine.add_rule(Arc::new(CrystalLoadCapacitorRule));
    engine.add_rule(Arc::new(PowerPinRule));
    engine.add_rule(Arc::new(ESDProtectionRule));
    engine.add_rule(Arc::new(BulkCapacitorRule));
    // Your rule goes here!
    engine
}
```

---

## Tutorial: Create Your First Rule

Let's build a real rule: **check that every LED has a series current-limiting resistor**.

An LED without a current-limiting resistor will draw excessive current and burn out.
This is one of the most common beginner mistakes in circuit design.

### Step 1: Create the Rule Struct

Open `src-tauri/src/analyzer/rules.rs` and add your struct after the existing rules
(before the `#[cfg(test)]` block):

```rust
pub struct LedSeriesResistorRule;
```

### Step 2: Implement the `Rule` Trait

```rust
impl Rule for LedSeriesResistorRule {
    fn id(&self) -> &str {
        "led_series_resistor"
    }

    fn name(&self) -> &str {
        "LED Series Resistor Check"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, schematic: &Schematic) -> Vec<Issue> {
        let mut issues = Vec::new();
        let all_components = get_all_components(schematic);

        // Find all LEDs: reference starts with 'D' and value suggests an LED
        let leds: Vec<&Component> = all_components
            .iter()
            .filter(|c| {
                let ref_upper = c.reference.to_uppercase();
                let value_upper = c.value.to_uppercase();
                let lib_upper = c.lib_id.to_uppercase();

                ref_upper.starts_with('D') && (
                    value_upper.contains("LED") ||
                    lib_upper.contains("LED") ||
                    value_upper.contains("RED") ||
                    value_upper.contains("GREEN") ||
                    value_upper.contains("BLUE") ||
                    value_upper.contains("YELLOW") ||
                    value_upper.contains("WHITE") ||
                    value_upper.contains("AMBER") ||
                    value_upper.contains("ORANGE")
                )
            })
            .copied()
            .collect();

        for led in leds {
            // Look for a resistor within 15mm of the LED
            let has_series_resistor = all_components.iter().any(|c| {
                (c.reference.starts_with('R') || c.reference.starts_with('r'))
                    && is_nearby(c, &led.position, 15.0)
            });

            if !has_series_resistor {
                issues.push(Issue {
                    id: uuid::Uuid::new_v4().to_string(),
                    rule_id: self.id().to_string(),
                    severity: self.severity(),
                    message: format!(
                        "LED {} ({}) has no series current-limiting resistor within 15mm",
                        led.reference, led.value
                    ),
                    component: Some(led.reference.clone()),
                    location: Some(led.position.clone()),
                    suggestion: Some(
                        "Add a series resistor to limit LED current. \
                         Typical values: 220-1k ohm for 3.3V, 330-1k ohm for 5V."
                            .to_string(),
                    ),
                    risk_score: None,
                });
            }
        }

        issues
    }
}
```

### Step 3: Register the Rule

In the same file, add your rule to `with_default_rules()`:

```rust
pub fn with_default_rules() -> Self {
    let mut engine = Self::new();
    engine.add_rule(Arc::new(DecouplingCapacitorRule));
    engine.add_rule(Arc::new(I2CPullResistorRule));
    engine.add_rule(Arc::new(CrystalLoadCapacitorRule));
    engine.add_rule(Arc::new(PowerPinRule));
    engine.add_rule(Arc::new(ESDProtectionRule));
    engine.add_rule(Arc::new(BulkCapacitorRule));
    engine.add_rule(Arc::new(LedSeriesResistorRule));  // <-- Add this line
    engine
}
```

### Step 4: Write Tests

Add tests to the `#[cfg(test)] mod tests` block at the bottom of `rules.rs`:

```rust
#[test]
fn test_led_series_resistor_missing() {
    let mut schematic = create_test_schematic();

    // Add an LED without a resistor
    schematic.components.push(Component {
        uuid: "d1".to_string(),
        reference: "D1".to_string(),
        value: "LED_Red".to_string(),
        lib_id: "Device:LED".to_string(),
        footprint: None,
        position: Position { x: 100.0, y: 100.0 },
        rotation: 0.0,
        properties: HashMap::new(),
        pins: Vec::new(),
    });

    let rule = LedSeriesResistorRule;
    let issues = rule.check(&schematic);
    assert!(!issues.is_empty(), "Should detect missing series resistor");
    assert_eq!(issues[0].rule_id, "led_series_resistor");
}

#[test]
fn test_led_series_resistor_present() {
    let mut schematic = create_test_schematic();

    // Add an LED
    schematic.components.push(Component {
        uuid: "d1".to_string(),
        reference: "D1".to_string(),
        value: "LED_Green".to_string(),
        lib_id: "Device:LED".to_string(),
        footprint: None,
        position: Position { x: 100.0, y: 100.0 },
        rotation: 0.0,
        properties: HashMap::new(),
        pins: Vec::new(),
    });

    // Add a series resistor nearby
    schematic.components.push(Component {
        uuid: "r1".to_string(),
        reference: "R1".to_string(),
        value: "330".to_string(),
        lib_id: "Device:R".to_string(),
        footprint: None,
        position: Position { x: 105.0, y: 100.0 },
        rotation: 0.0,
        properties: HashMap::new(),
        pins: Vec::new(),
    });

    let rule = LedSeriesResistorRule;
    let issues = rule.check(&schematic);
    assert!(issues.is_empty(), "Should not flag when resistor is nearby");
}
```

### Step 5: Run Tests

```bash
cd src-tauri
cargo test
```

All existing tests should still pass, and your new tests should pass too.

### Step 6: Open a Pull Request

Push your branch and open a PR. In the description, mention which `good-first-issue`
you are addressing (if applicable).

---

## Good First Issues

The following rules are not yet implemented and are tagged as `good-first-issue` on
GitHub. Each one follows the exact same pattern as the tutorial above.

### Schematic Rules (implement in `src-tauri/src/analyzer/rules.rs`)

| Rule Idea | Description | Difficulty |
|-----------|-------------|------------|
| **LED Series Resistor** | Check that LEDs (D* with LED in value/lib) have a nearby resistor | Easy |
| **Reset Pin Pull-up** | Check that MCU RESET/NRST pins have a pull-up resistor (10k-100k) | Easy |
| **Unused IC Pins** | Warn when an IC has unconnected pins that should be tied high/low | Medium |
| **SPI Bus Completeness** | If MOSI/MISO/SCK labels exist, verify all three are present plus CS | Easy |
| **UART TX/RX Pairing** | If TX label exists, check that a corresponding RX exists (and vice versa) | Easy |
| **Voltage Regulator Input Cap** | Check that voltage regulators have an input capacitor (not just output) | Easy |
| **Antenna Matching Network** | Check that RF components (antenna, balun) have nearby matching components | Medium |
| **Motor Driver Flyback Diode** | Check that motor/relay drivers have flyback protection diodes | Medium |
| **Connector Pin 1 Marking** | Verify that multi-pin connectors have Pin 1 clearly identified | Medium |
| **Power LED Indicator** | Suggest adding a power indicator LED if the board has a regulator but no LED | Easy |

### PCB Compliance Rules (implement in `src-tauri/src/compliance/rules.rs`)

These follow a different pattern (JSON-based `CustomRule` / `RuleCheck` enum) but are
equally approachable:

| Rule Idea | Description | Difficulty |
|-----------|-------------|------------|
| **Silkscreen Over Pad** | Check that silkscreen does not overlap exposed pads | Medium |
| **Thermal Relief on Ground Plane** | Verify thermal relief settings on ground zone connections | Medium |
| **Testpoint Accessibility** | Check that test points are accessible and not blocked by components | Medium |

---

## Development Setup

### Prerequisites

- **Rust** (latest stable) - [Install via rustup](https://rustup.rs/)
- **Node.js 18+** and npm
- **Tauri v2 system dependencies** - [Platform-specific guide](https://v2.tauri.app/start/prerequisites/)

### Building

```bash
# Install frontend dependencies
npm install

# Run in development mode (frontend + backend hot reload)
npm run tauri dev

# Run Rust tests only (from src-tauri/)
cd src-tauri && cargo test

# Build production binary
npm run tauri build
```

### Project Structure

```
src-tauri/src/
  analyzer/
    rules.rs          <-- Schematic rules engine (START HERE)
    capacitor_classifier.rs
    decoupling_groups.rs
    drs.rs            <-- Decoupling Risk Scoring
    explanations.rs   <-- Issue explanations (What/Why/Fix)
  compliance/
    rules.rs          <-- PCB compliance rules (JSON-based)
    ipc2221.rs        <-- IPC-2221 current capacity
    emi.rs            <-- EMI analysis
  parser/
    kicad.rs          <-- KiCad 6-9 schematic parser
    kicad_legacy.rs   <-- KiCad 4-5 legacy parser
    pcb.rs            <-- PCB layout parser
    schema.rs         <-- Core data types (Schematic, Component, etc.)
  ucs/                <-- Unified Circuit Schema (graph model)
  ai/                 <-- AI integration (Claude, Ollama)
  datasheets/         <-- Datasheet compliance checking

src/                  <-- React/TypeScript frontend
  components/         <-- UI panels (Issues, DRS, Compliance, etc.)
  lib/                <-- State management, API calls
```

---

## Code Style & Conventions

### Rust

- Follow standard `rustfmt` formatting (`cargo fmt` before committing).
- Use `cargo clippy` to catch common issues.
- Every rule must have at least two tests: one where the issue **is** detected and one
  where it **is not** (i.e. the design is correct).
- Use `Severity::Error` for issues that will cause hardware failure, `Severity::Warning`
  for likely problems, `Severity::Info` for suggestions, and `Severity::Suggestion` for
  nice-to-haves.
- Prefer helper functions that already exist (`get_all_components`, `is_nearby`,
  `parse_value`, `value_matches_pattern`) over writing new ones.

### TypeScript / React

- Use functional components with hooks.
- State management via Zustand (see `src/lib/store.ts`).
- Icons from Lucide React only (no custom icon files).
- Follow the existing Tailwind CSS patterns for styling.

### Commits

- Use clear, imperative-mood commit messages: "Add LED series resistor rule" not
  "Added LED series resistor rule".
- One logical change per commit.
- Reference the issue number if applicable: "Add LED series resistor rule (#42)".

---

## Submitting a Pull Request

1. **Fork** the repository and create a feature branch from `main`.
2. **Implement** your rule following the tutorial pattern.
3. **Write tests** (minimum: one failing-case test, one passing-case test).
4. **Run the full test suite**: `cd src-tauri && cargo test`
5. **Format your code**: `cargo fmt`
6. **Open a PR** with:
   - A clear title describing the rule.
   - A brief explanation of what the rule checks and why it matters.
   - A reference to the `good-first-issue` if applicable.

### PR Checklist

- [ ] New rule struct implements the `Rule` trait
- [ ] Rule is registered in `with_default_rules()`
- [ ] At least two tests (issue detected + issue not detected)
- [ ] `cargo test` passes
- [ ] `cargo fmt` applied
- [ ] `cargo clippy` has no new warnings

---

## Questions?

If you have questions about a specific rule idea or need help with the codebase,
open a [Discussion](https://github.com/your-org/designguard/discussions) or comment
on the relevant issue. We're happy to help first-time contributors get started.
