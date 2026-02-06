#!/usr/bin/env python3
"""
KiCad S-Expression Parser Edge-Case Test Generator

Generates 20 edge-case .kicad_sch files for unit testing the parser.
Each file tests a specific edge case scenario.
"""

import os
import uuid
from pathlib import Path


def escape_string(s: str) -> str:
    """Escape special characters in S-expression strings."""
    return s.replace('\\', '\\\\').replace('"', '\\"')


def generate_uuid() -> str:
    """Generate a valid UUID v4."""
    return str(uuid.uuid4())


def generate_base_schematic() -> str:
    """Generate base schematic structure."""
    return f"""(kicad_sch (version 20231120) (generator "eeschema") (generator_version "8.0")
  (uuid "{generate_uuid()}")
  (paper "A4")
  (lib_symbols)
"""


def case_1_deeply_nested_hierarchical_sheets() -> str:
    """Case 1: Deeply nested hierarchical sheets."""
    return generate_base_schematic() + f"""
  ; Deeply nested hierarchical structure
  (sheet (at 50 50 0) (size 100 80)
    (uuid "{generate_uuid()}")
    (property "Sheetname" "Level1" (at 50 50 0))
    (property "Sheetfile" "level1.kicad_sch" (at 50 50 0))
    (pin "1" (uuid "{generate_uuid()}"))
    (sheet (at 60 60 0) (size 80 60)
      (uuid "{generate_uuid()}")
      (property "Sheetname" "Level2" (at 60 60 0))
      (property "Sheetfile" "level2.kicad_sch" (at 60 60 0))
      (pin "1" (uuid "{generate_uuid()}"))
      (sheet (at 70 70 0) (size 60 40)
        (uuid "{generate_uuid()}")
        (property "Sheetname" "Level3" (at 70 70 0))
        (property "Sheetfile" "level3.kicad_sch" (at 70 70 0))
        (pin "1" (uuid "{generate_uuid()}"))
      )
    )
  )
  (symbol (lib_id "Device:R") (at 150 100 0) (unit 1)
    (in_bom yes) (on_board yes) (dnp no)
    (uuid "{generate_uuid()}")
    (property "Reference" "R1" (at 151.6 98.53 0))
    (property "Value" "10k" (at 151.6 101.07 0))
    (pin "1" (uuid "{generate_uuid()}"))
    (pin "2" (uuid "{generate_uuid()}"))
  )
)
"""


def case_2_missing_timestamps() -> str:
    """Case 2: Missing timestamps and optional metadata."""
    return f"""(kicad_sch (version 20231120)
  (uuid "{generate_uuid()}")
  (paper "A4")
  (lib_symbols)
  ; Missing generator and generator_version
  (symbol (lib_id "Device:C") (at 100 100 0) (unit 1)
    (in_bom yes) (on_board yes) (dnp no)
    ; Missing uuid on symbol
    (property "Reference" "C1" (at 101.6 98.53 0))
    (property "Value" "100nF" (at 101.6 101.07 0))
    (pin "1" (uuid "{generate_uuid()}"))
    (pin "2" (uuid "{generate_uuid()}"))
  )
)
"""


def case_3_custom_footprints_special_chars() -> str:
    """Case 3: Custom footprints with special characters."""
    return generate_base_schematic() + f"""
  (symbol (lib_id "Device:R") (at 100 100 0) (unit 1)
    (in_bom yes) (on_board yes) (dnp no)
    (uuid "{generate_uuid()}")
    (property "Reference" "R1" (at 101.6 98.53 0))
    (property "Value" "10k" (at 101.6 101.07 0))
    (property "Footprint" "Custom Footprint:My Part_v2.0" (at 98.552 100 90))
    (pin "1" (uuid "{generate_uuid()}"))
    (pin "2" (uuid "{generate_uuid()}"))
  )
  (symbol (lib_id "Device:R") (at 150 100 0) (unit 1)
    (in_bom yes) (on_board yes) (dnp no)
    (uuid "{generate_uuid()}")
    (property "Reference" "R2" (at 151.6 98.53 0))
    (property "Value" "100Ω" (at 151.6 101.07 0))
    (property "Footprint" "Footprint:电阻器_100Ω" (at 148.552 100 90))
    (pin "1" (uuid "{generate_uuid()}"))
    (pin "2" (uuid "{generate_uuid()}"))
  )
  (symbol (lib_id "Device:C") (at 200 100 0) (unit 1)
    (in_bom yes) (on_board yes) (dnp no)
    (uuid "{generate_uuid()}")
    (property "Reference" "C1" (at 201.6 98.53 0))
    (property "Value" "100nF" (at 201.6 101.07 0))
    (property "Footprint" "Footprint:Part#123-ABC/XYZ" (at 198.552 100 90))
    (pin "1" (uuid "{generate_uuid()}"))
    (pin "2" (uuid "{generate_uuid()}"))
  )
)
"""


def case_4_empty_property_values() -> str:
    """Case 4: Empty property values."""
    return generate_base_schematic() + f"""
  (symbol (lib_id "Device:R") (at 100 100 0) (unit 1)
    (in_bom yes) (on_board yes) (dnp no)
    (uuid "{generate_uuid()}")
    (property "Reference" "" (at 101.6 98.53 0))
    (property "Value" "10k" (at 101.6 101.07 0))
    (property "Footprint" "" (at 98.552 100 90))
    (pin "1" (uuid "{generate_uuid()}"))
    (pin "2" (uuid "{generate_uuid()}"))
  )
  (symbol (lib_id "Device:C") (at 150 100 0) (unit 1)
    (in_bom yes) (on_board yes) (dnp no)
    (uuid "{generate_uuid()}")
    ; Missing Reference property
    (property "Value" "100nF" (at 151.6 101.07 0))
    (pin "1" (uuid "{generate_uuid()}"))
    (pin "2" (uuid "{generate_uuid()}"))
  )
)
"""


def case_5_extreme_coordinate_values() -> str:
    """Case 5: Extreme coordinate values."""
    return generate_base_schematic() + f"""
  (symbol (lib_id "Device:R") (at 999999.9999 999999.9999 0) (unit 1)
    (in_bom yes) (on_board yes) (dnp no)
    (uuid "{generate_uuid()}")
    (property "Reference" "R1" (at 1000000.9999 999998.9999 0))
    (property "Value" "10k" (at 1000000.9999 1000001.9999 0))
    (pin "1" (uuid "{generate_uuid()}"))
    (pin "2" (uuid "{generate_uuid()}"))
  )
  (symbol (lib_id "Device:C") (at -100.5 -200.3 0) (unit 1)
    (in_bom yes) (on_board yes) (dnp no)
    (uuid "{generate_uuid()}")
    (property "Reference" "C1" (at -99.5 -201.3 0))
    (property "Value" "100nF" (at -99.5 -198.3 0))
    (pin "1" (uuid "{generate_uuid()}"))
    (pin "2" (uuid "{generate_uuid()}"))
  )
  (symbol (lib_id "Device:L") (at 0 0 0) (unit 1)
    (in_bom yes) (on_board yes) (dnp no)
    (uuid "{generate_uuid()}")
    (property "Reference" "L1" (at 1.6 -1.47 0))
    (property "Value" "10uH" (at 1.6 1.07 0))
    (pin "1" (uuid "{generate_uuid()}"))
    (pin "2" (uuid "{generate_uuid()}"))
  )
)
"""


def case_6_malformed_uuids() -> str:
    """Case 6: Malformed but parseable UUIDs."""
    return generate_base_schematic() + f"""
  (symbol (lib_id "Device:R") (at 100 100 0) (unit 1)
    (in_bom yes) (on_board yes) (dnp no)
    (uuid "not-a-uuid")
    (property "Reference" "R1" (at 101.6 98.53 0))
    (property "Value" "10k" (at 101.6 101.07 0))
    (pin "1" (uuid "also-not-a-uuid"))
    (pin "2" (uuid "{generate_uuid()}"))
  )
  (symbol (lib_id "Device:C") (at 150 100 0) (unit 1)
    (in_bom yes) (on_board yes) (dnp no)
    ; Missing uuid
    (property "Reference" "C1" (at 151.6 98.53 0))
    (property "Value" "100nF" (at 151.6 101.07 0))
    (pin "1" (uuid "{generate_uuid()}"))
    (pin "2" (uuid "{generate_uuid()}"))
  )
  (symbol (lib_id "Device:L") (at 200 100 0) (unit 1)
    (in_bom yes) (on_board yes) (dnp no)
    (uuid "{generate_uuid()}")
    (property "Reference" "L1" (at 201.6 98.53 0))
    (property "Value" "10uH" (at 201.6 101.07 0))
    ; Duplicate UUID from R1
    (pin "1" (uuid "not-a-uuid"))
    (pin "2" (uuid "{generate_uuid()}"))
  )
)
"""


def case_7_unicode_in_values() -> str:
    """Case 7: Unicode in component values."""
    return generate_base_schematic() + f"""
  (symbol (lib_id "Device:Battery") (at 100 100 0) (unit 1)
    (in_bom yes) (on_board yes) (dnp no)
    (uuid "{generate_uuid()}")
    (property "Reference" "B1" (at 101.6 98.53 0))
    (property "Value" "🔋 Battery 3.7V" (at 101.6 101.07 0))
    (pin "1" (uuid "{generate_uuid()}"))
    (pin "2" (uuid "{generate_uuid()}"))
  )
  (symbol (lib_id "Device:R") (at 150 100 0) (unit 1)
    (in_bom yes) (on_board yes) (dnp no)
    (uuid "{generate_uuid()}")
    (property "Reference" "R1" (at 151.6 98.53 0))
    (property "Value" "电阻器 10kΩ" (at 151.6 101.07 0))
    (pin "1" (uuid "{generate_uuid()}"))
    (pin "2" (uuid "{generate_uuid()}"))
  )
  (symbol (lib_id "Device:R") (at 200 100 0) (unit 1)
    (in_bom yes) (on_board yes) (dnp no)
    (uuid "{generate_uuid()}")
    (property "Reference" "R2" (at 201.6 98.53 0))
    (property "Value" "Резистор 100Ω" (at 201.6 101.07 0))
    (pin "1" (uuid "{generate_uuid()}"))
    (pin "2" (uuid "{generate_uuid()}"))
  )
)
"""


def case_8_very_long_strings() -> str:
    """Case 8: Very long strings."""
    long_ref = "R" + "1" * 200
    long_value = "Resistor_" + "X" * 500 + "_10kΩ"
    long_footprint = "Footprint:" + "A" * 300 + ":Custom_Part"
    
    return generate_base_schematic() + f"""
  (symbol (lib_id "Device:R") (at 100 100 0) (unit 1)
    (in_bom yes) (on_board yes) (dnp no)
    (uuid "{generate_uuid()}")
    (property "Reference" "{long_ref}" (at 101.6 98.53 0))
    (property "Value" "{long_value}" (at 101.6 101.07 0))
    (property "Footprint" "{long_footprint}" (at 98.552 100 90))
    (pin "1" (uuid "{generate_uuid()}"))
    (pin "2" (uuid "{generate_uuid()}"))
  )
)
"""


def case_9_missing_required_fields() -> str:
    """Case 9: Missing required fields."""
    return generate_base_schematic() + f"""
  ; Symbol without lib_id
  (symbol (at 100 100 0) (unit 1)
    (in_bom yes) (on_board yes) (dnp no)
    (uuid "{generate_uuid()}")
    (property "Reference" "R1" (at 101.6 98.53 0))
    (property "Value" "10k" (at 101.6 101.07 0))
    (pin "1" (uuid "{generate_uuid()}"))
    (pin "2" (uuid "{generate_uuid()}"))
  )
  ; Component without Reference property
  (symbol (lib_id "Device:C") (at 150 100 0) (unit 1)
    (in_bom yes) (on_board yes) (dnp no)
    (uuid "{generate_uuid()}")
    (property "Value" "100nF" (at 151.6 101.07 0))
    (pin "1" (uuid "{generate_uuid()}"))
    (pin "2" (uuid "{generate_uuid()}"))
  )
  ; Wire without pts
  (wire (uuid "{generate_uuid()}"))
)
"""


def case_10_nested_comments() -> str:
    """Case 10: Nested comments."""
    return generate_base_schematic() + f"""
  ; Top level comment
  (symbol (lib_id "Device:R") (at 100 100 0) (unit 1)
    ; Comment inside symbol
    (in_bom yes) (on_board yes) (dnp no)
    (uuid "{generate_uuid()}")
    (property "Reference" "R1" (at 101.6 98.53 0))
    ; Comment with "quotes" and (parens)
    (property "Value" "10k" (at 101.6 101.07 0))
    (pin "1" (uuid "{generate_uuid()}"))
    (pin "2" (uuid "{generate_uuid()}"))
  )
  ; Another comment
  ; Multi-line
  ; comment block
  (wire (pts (xy 90 50.8) (xy 98.33 50.8))
    (uuid "{generate_uuid()}")
  )
)
"""


def case_11_empty_lists() -> str:
    """Case 11: Empty lists and collections."""
    return f"""(kicad_sch (version 20231120) (generator "eeschema") (generator_version "8.0")
  (uuid "{generate_uuid()}")
  (paper "A4")
  ; Empty lib_symbols
  (lib_symbols)
  ; Symbol with no pins
  (symbol (lib_id "Device:R") (at 100 100 0) (unit 1)
    (in_bom yes) (on_board yes) (dnp no)
    (uuid "{generate_uuid()}")
    (property "Reference" "R1" (at 101.6 98.53 0))
    (property "Value" "10k" (at 101.6 101.07 0))
  )
  ; Wire with empty point list (malformed but parseable)
  (wire (pts)
    (uuid "{generate_uuid()}")
  )
)
"""


def case_12_mixed_case_formatting() -> str:
    """Case 12: Mixed case and formatting."""
    return f"""(KICAD_SCH (VERSION 20231120) (GENERATOR "eeschema") (GENERATOR_VERSION "8.0")
  (UUID "{generate_uuid()}")
  (PAPER "A4")
  (LIB_SYMBOLS)
  (SYMBOL (LIB_ID "Device:R") (AT 100 100 0) (UNIT 1)
    (IN_BOM yes) (ON_BOARD yes) (DNP no)
    (UUID "{generate_uuid()}")
    (PROPERTY "Reference" "R1" (AT 101.6 98.53 0))
    (PROPERTY "Value" "10k" (AT 101.6 101.07 0))
    (PIN "1" (UUID "{generate_uuid()}"))
    (PIN "2" (UUID "{generate_uuid()}"))
  )
  (symbol (lib_id "Device:C") (at 150 100 0) (unit 1)
    (in_bom yes) (on_board yes) (dnp no)
    (uuid "{generate_uuid()}")
    (property "Reference" "C1" (at 151.6 98.53 0))
    (property "Value" "100nF" (at 151.6 101.07 0))
    (pin "1" (uuid "{generate_uuid()}"))
    (pin "2" (uuid "{generate_uuid()}"))
  )
)
"""


def case_13_legacy_format() -> str:
    """Case 13: Legacy format compatibility."""
    return f"""(kicad_sch (version 20211014) (generator "eeschema") (generator_version "6.0")
  (uuid "{generate_uuid()}")
  (paper "A4")
  (lib_symbols)
  (symbol (lib_id "Device:R") (at 100 100 0) (unit 1)
    (in_bom yes) (on_board yes) (dnp no)
    (uuid "{generate_uuid()}")
    (property "Reference" "R1" (at 101.6 98.53 0))
    (property "Value" "10k" (at 101.6 101.07 0))
    (pin "1" (uuid "{generate_uuid()}"))
    (pin "2" (uuid "{generate_uuid()}"))
  )
)
"""


def case_14_escaped_strings() -> str:
    """Case 14: Escaped strings."""
    return generate_base_schematic() + f"""
  (symbol (lib_id "Device:R") (at 100 100 0) (unit 1)
    (in_bom yes) (on_board yes) (dnp no)
    (uuid "{generate_uuid()}")
    (property "Reference" "R1" (at 101.6 98.53 0))
    (property "Value" "Part with \\"quotes\\" inside" (at 101.6 101.07 0))
    (property "Footprint" "Footprint:Part\\\\with\\\\backslashes" (at 98.552 100 90))
    (pin "1" (uuid "{generate_uuid()}"))
    (pin "2" (uuid "{generate_uuid()}"))
  )
)
"""


def case_15_boundary_values() -> str:
    """Case 15: Boundary value testing."""
    return generate_base_schematic() + f"""
  (symbol (lib_id "Device:R") (at 123.4567 789.0123 0) (unit 1)
    (in_bom yes) (on_board yes) (dnp no)
    (uuid "{generate_uuid()}")
    (property "Reference" "R1" (at 124.4567 788.0123 0))
    (property "Value" "10k" (at 124.4567 790.0123 0))
    (pin "1" (uuid "{generate_uuid()}"))
    (pin "2" (uuid "{generate_uuid()}"))
  )
  (symbol (lib_id "Device:C") (at 0.0001 0.0001 0) (unit 1)
    (in_bom yes) (on_board yes) (dnp no)
    (uuid "{generate_uuid()}")
    (property "Reference" "C1" (at 1.0001 -0.9999 0))
    (property "Value" "100nF" (at 1.0001 1.0001 0))
    (pin "1" (uuid "{generate_uuid()}"))
    (pin "2" (uuid "{generate_uuid()}"))
  )
)
"""


def case_16_duplicate_references() -> str:
    """Case 16: Duplicate references."""
    return generate_base_schematic() + f"""
  (symbol (lib_id "Device:R") (at 100 100 0) (unit 1)
    (in_bom yes) (on_board yes) (dnp no)
    (uuid "{generate_uuid()}")
    (property "Reference" "R1" (at 101.6 98.53 0))
    (property "Value" "10k" (at 101.6 101.07 0))
    (pin "1" (uuid "{generate_uuid()}"))
    (pin "2" (uuid "{generate_uuid()}"))
  )
  (symbol (lib_id "Device:R") (at 150 100 0) (unit 1)
    (in_bom yes) (on_board yes) (dnp no)
    (uuid "{generate_uuid()}")
    (property "Reference" "R1" (at 151.6 98.53 0))
    (property "Value" "20k" (at 151.6 101.07 0))
    (pin "1" (uuid "{generate_uuid()}"))
    (pin "2" (uuid "{generate_uuid()}"))
  )
  (symbol (lib_id "Device:C") (at 200 100 0) (unit 1)
    (in_bom yes) (on_board yes) (dnp no)
    (uuid "{generate_uuid()}")
    (property "Reference" "R1" (at 201.6 98.53 0))
    (property "Value" "100nF" (at 201.6 101.07 0))
    (pin "1" (uuid "{generate_uuid()}"))
    (pin "2" (uuid "{generate_uuid()}"))
  )
)
"""


def case_17_complex_pin_configurations() -> str:
    """Case 17: Complex pin configurations."""
    pins = []
    for i in range(1, 101):  # 100 pins
        pins.append(f'    (pin "{i}" (uuid "{generate_uuid()}"))')
    
    return generate_base_schematic() + f"""
  (symbol (lib_id "MCU:STM32F4") (at 100 100 0) (unit 1)
    (in_bom yes) (on_board yes) (dnp no)
    (uuid "{generate_uuid()}")
    (property "Reference" "U1" (at 101.6 98.53 0))
    (property "Value" "STM32F401" (at 101.6 101.07 0))
{chr(10).join(pins)}
  )
  (symbol (lib_id "Device:Connector") (at 200 100 0) (unit 1)
    (in_bom yes) (on_board yes) (dnp no)
    (uuid "{generate_uuid()}")
    (property "Reference" "J1" (at 201.6 98.53 0))
    (property "Value" "Connector" (at 201.6 101.07 0))
    (pin "VCC" (uuid "{generate_uuid()}"))
    (pin "GND" (uuid "{generate_uuid()}"))
    (pin "DATA+" (uuid "{generate_uuid()}"))
    (pin "DATA-" (uuid "{generate_uuid()}"))
  )
  (symbol (lib_id "Device:R") (at 250 100 0) (unit 1)
    (in_bom yes) (on_board yes) (dnp no)
    (uuid "{generate_uuid()}")
    (property "Reference" "R1" (at 251.6 98.53 0))
    (property "Value" "10k" (at 251.6 101.07 0))
    ; Missing pin UUID
    (pin "1")
    (pin "2" (uuid "{generate_uuid()}"))
  )
)
"""


def case_18_invalid_property_formats() -> str:
    """Case 18: Invalid property formats."""
    return generate_base_schematic() + f"""
  (symbol (lib_id "Device:R") (at 100 100 0) (unit 1)
    (in_bom yes) (on_board yes) (dnp no)
    (uuid "{generate_uuid()}")
    ; Property with wrong number of arguments
    (property "Reference" (at 101.6 98.53 0))
    (property "Value" "10k" (at 101.6 101.07 0))
    ; Property without quotes
    (property Reference R1 (at 101.6 98.53 0))
    (pin "1" (uuid "{generate_uuid()}"))
    (pin "2" (uuid "{generate_uuid()}"))
  )
)
"""


def case_19_sheet_instances_hierarchical_labels() -> str:
    """Case 19: Sheet instances and hierarchical labels."""
    return generate_base_schematic() + f"""
  (sheet (at 50 50 0) (size 100 80)
    (uuid "{generate_uuid()}")
    (property "Sheetname" "SubSheet1" (at 50 50 0))
    (property "Sheetfile" "subsheet1.kicad_sch" (at 50 50 0))
    (pin "1" (uuid "{generate_uuid()}"))
  )
  (global_label "SDA" (at 200.0 80.0 0) (shape bidirectional)
    (effects (font (size 1.27 1.27)) (justify left))
    (uuid "{generate_uuid()}")
  )
  (global_label "SCL" (at 200.0 90.0 0) (shape bidirectional)
    (effects (font (size 1.27 1.27)) (justify left))
    (uuid "{generate_uuid()}")
  )
  (hierarchical_label "CLK_IN" (at 150 100 0) (shape input)
    (effects (font (size 1.27 1.27)) (justify left))
    (uuid "{generate_uuid()}")
  )
  (hierarchical_label "DATA_OUT" (at 150 110 0) (shape output)
    (effects (font (size 1.27 1.27)) (justify left))
    (uuid "{generate_uuid()}")
  )
  (symbol (lib_id "Device:R") (at 250 100 0) (unit 1)
    (in_bom yes) (on_board yes) (dnp no)
    (uuid "{generate_uuid()}")
    (property "Reference" "R1" (at 251.6 98.53 0))
    (property "Value" "10k" (at 251.6 101.07 0))
    (pin "1" (uuid "{generate_uuid()}"))
    (pin "2" (uuid "{generate_uuid()}"))
  )
)
"""


def case_20_mixed_valid_invalid() -> str:
    """Case 20: Mixed valid and invalid elements."""
    return generate_base_schematic() + f"""
  ; Valid symbol
  (symbol (lib_id "Device:R") (at 100 100 0) (unit 1)
    (in_bom yes) (on_board yes) (dnp no)
    (uuid "{generate_uuid()}")
    (property "Reference" "R1" (at 101.6 98.53 0))
    (property "Value" "10k" (at 101.6 101.07 0))
    (pin "1" (uuid "{generate_uuid()}"))
    (pin "2" (uuid "{generate_uuid()}"))
  )
  ; Malformed symbol (missing lib_id)
  (symbol (at 150 100 0) (unit 1)
    (in_bom yes) (on_board yes) (dnp no)
    (uuid "{generate_uuid()}")
    (property "Reference" "R2" (at 151.6 98.53 0))
  )
  ; Valid wire
  (wire (pts (xy 90 50.8) (xy 98.33 50.8))
    (uuid "{generate_uuid()}")
  )
  ; Partial wire (missing pts)
  (wire (uuid "{generate_uuid()}"))
  ; Valid symbol
  (symbol (lib_id "Device:C") (at 200 100 0) (unit 1)
    (in_bom yes) (on_board yes) (dnp no)
    (uuid "{generate_uuid()}")
    (property "Reference" "C1" (at 201.6 98.53 0))
    (property "Value" "100nF" (at 201.6 101.07 0))
    (pin "1" (uuid "{generate_uuid()}"))
    (pin "2" (uuid "{generate_uuid()}"))
  )
  ; Incomplete component (missing properties)
  (symbol (lib_id "Device:L") (at 250 100 0) (unit 1)
    (in_bom yes) (on_board yes) (dnp no)
    (uuid "{generate_uuid()}")
  )
)
"""


def main():
    """Generate all edge-case test files."""
    output_dir = Path(__file__).parent / "edge-cases"
    output_dir.mkdir(parents=True, exist_ok=True)
    
    cases = [
        ("case_01_deeply_nested_hierarchical_sheets.kicad_sch", case_1_deeply_nested_hierarchical_sheets),
        ("case_02_missing_timestamps.kicad_sch", case_2_missing_timestamps),
        ("case_03_custom_footprints_special_chars.kicad_sch", case_3_custom_footprints_special_chars),
        ("case_04_empty_property_values.kicad_sch", case_4_empty_property_values),
        ("case_05_extreme_coordinate_values.kicad_sch", case_5_extreme_coordinate_values),
        ("case_06_malformed_uuids.kicad_sch", case_6_malformed_uuids),
        ("case_07_unicode_in_values.kicad_sch", case_7_unicode_in_values),
        ("case_08_very_long_strings.kicad_sch", case_8_very_long_strings),
        ("case_09_missing_required_fields.kicad_sch", case_9_missing_required_fields),
        ("case_10_nested_comments.kicad_sch", case_10_nested_comments),
        ("case_11_empty_lists.kicad_sch", case_11_empty_lists),
        ("case_12_mixed_case_formatting.kicad_sch", case_12_mixed_case_formatting),
        ("case_13_legacy_format.kicad_sch", case_13_legacy_format),
        ("case_14_escaped_strings.kicad_sch", case_14_escaped_strings),
        ("case_15_boundary_values.kicad_sch", case_15_boundary_values),
        ("case_16_duplicate_references.kicad_sch", case_16_duplicate_references),
        ("case_17_complex_pin_configurations.kicad_sch", case_17_complex_pin_configurations),
        ("case_18_invalid_property_formats.kicad_sch", case_18_invalid_property_formats),
        ("case_19_sheet_instances_hierarchical_labels.kicad_sch", case_19_sheet_instances_hierarchical_labels),
        ("case_20_mixed_valid_invalid.kicad_sch", case_20_mixed_valid_invalid),
    ]
    
    print(f"Generating {len(cases)} edge-case test files...")
    for filename, generator_func in cases:
        filepath = output_dir / filename
        content = generator_func()
        filepath.write_text(content, encoding='utf-8')
        print(f"  ✓ Generated {filename}")
    
    print(f"\n✓ All test files generated in: {output_dir}")
    print(f"  Total files: {len(cases)}")


if __name__ == "__main__":
    main()
