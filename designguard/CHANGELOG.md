# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2025-02-07

### Added

- Initial release of the DesignGuard library
- Schematic validation: decoupling capacitors, I2C pull-ups, crystal load caps, power pins, ESD, bulk caps
- PCB validation: EMI analysis, trace width (IPC-2221)
- Datasheet compliance checking (built-in and user JSON)
- Unified Circuit Schema (UCS) and KiCad adapter
- `DesignGuardCore::validate_schematic`, `validate_pcb`, `validate_project`
- `ValidationOptions`, `ValidationResult`, `ValidationStats`
- `parse_schematic`, `parse_pcb` convenience functions
- `discover_kicad_files` for project discovery
- Prelude module for convenient imports
