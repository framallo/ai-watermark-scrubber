//! scrub: detect and clean invisible-Unicode carriers and stray provenance
//! metadata (including multi-vendor AI text watermarks) from text and EPUB
//! files.
//!
//! Two ways to use it:
//!
//! * **CLI** — `scrub detect <path>` / `scrub clean <path>` (see the `scrub`
//!   binary).
//! * **Library** — the high-level [`detect_file`], [`clean_file`] and
//!   [`clean_file_in_place`] entry points, or the lower-level [`unicode`] and
//!   [`epub`] modules for in-memory work.
//!
//! ```no_run
//! use std::path::Path;
//! use scrub::{clean_file_in_place, detect_file, CleanOptions};
//!
//! let report = detect_file(Path::new("draft.md"))?;
//! if !report.is_clean() {
//!     clean_file_in_place(Path::new("draft.md"), CleanOptions::default())?;
//! }
//! # Ok::<(), anyhow::Error>(())
//! ```
//!
//! In-memory, no filesystem:
//!
//! ```
//! use scrub::{clean_text, CleanOptions};
//! let (clean, stats) = clean_text("in\u{200B}visible\u{00A0}text", CleanOptions::default());
//! assert_eq!(clean, "invisible text");
//! assert_eq!(stats.removed_count, 1); // the zero-width space
//! assert_eq!(stats.replaced_count, 1); // the no-break space folded to U+0020
//! ```

pub mod api;
pub mod epub;
pub mod unicode;

pub use api::{
    clean_file, clean_file_in_place, cleaned_path, detect_file, kind_of, CleanReport, DetectReport,
    Kind,
};
pub use unicode::{clean_text, inspect_text, CleanOptions, CleanStats, InspectReport};
