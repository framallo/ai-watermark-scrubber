//! scrub: detect and clean invisible-Unicode carriers and stray provenance
//! metadata from text and EPUB files.

pub mod epub;
pub mod unicode;

pub use unicode::{clean_text, inspect_text, CleanOptions, CleanStats, InspectReport};
