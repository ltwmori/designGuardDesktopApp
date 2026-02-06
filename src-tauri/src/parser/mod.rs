pub mod kicad;
pub mod schema;
pub mod sexp;
pub mod pcb;
pub mod pcb_schema;
pub mod netlist;
pub mod kicad_legacy;
pub mod format_detector;

// Re-export for convenience
pub use kicad::{KicadParser, KicadParseError};
pub use schema::*;
pub use sexp::{SExp, SExpParser, ParseError};
pub use pcb::{PcbParser, PcbParseError};
pub use pcb_schema::*;
pub use format_detector::{KicadVersion, detect_format, detect_and_parse_schematic, detect_and_parse_pcb};
pub use kicad_legacy::LegacyParser;