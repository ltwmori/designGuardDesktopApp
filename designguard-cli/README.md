# designguard-cli

[![Crates.io](https://img.shields.io/crates/v/designguard-cli.svg)](https://crates.io/crates/designguard-cli)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Command-line interface for [DesignGuard](https://github.com/ltwmori/designGuardDesktopApp): validate KiCad schematics and PCB files from the terminal or CI/CD.

## Installation

From crates.io:

```bash
cargo install designguard-cli
```

Or build from the [repository](https://github.com/ltwmori/designGuardDesktopApp) (requires the `designguard` crate in the same workspace):

```bash
git clone https://github.com/ltwmori/designGuardDesktopApp.git
cd designGuardDesktopApp
cargo build --release -p designguard-cli
```

The binary will be at `target/release/designguard-cli`.

## Usage

### Validate a single file

```bash
designguard-cli check design.kicad_sch
designguard-cli check board.kicad_pcb --no-ai
```

### Validate a whole project

```bash
designguard-cli project .
designguard-cli project /path/to/kicad/project --format json --no-ai
```

### List validation rules

```bash
designguard-cli rules
designguard-cli rules --verbose
```

## Options

| Option | Description |
|--------|-------------|
| `--format human \| json \| github \| gitlab` | Output format (default: `human`). Use `github` or `gitlab` for CI annotations. |
| `--fail-on critical \| high \| medium \| low \| info` | Exit with non-zero if issues at this severity or higher are found. |
| `--no-ai` | Disable AI and datasheet features (offline mode, recommended for CI). |
| `--strict` | Enable all validation rules. |

## CI/CD

Use `--no-ai` and `--fail-on` in pipelines:

```bash
designguard-cli project . --format github --fail-on high --no-ai
```

See [.github/workflows/designguard.yml](https://github.com/ltwmori/designGuardDesktopApp/blob/main/.github/workflows/designguard.yml) in the repo for a GitHub Actions example.

## Supported files

- `.kicad_sch`, `.sch` (schematic)
- `.kicad_pcb`, `.brd` (PCB)

## License

MIT. See [LICENSE](https://github.com/ltwmori/designGuardDesktopApp/blob/main/LICENSE) in the repository root.
