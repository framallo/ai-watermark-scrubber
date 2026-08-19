//! High-level, file-oriented API shared by the CLI and library consumers.
//!
//! The lower-level building blocks live in [`crate::unicode`] and
//! [`crate::epub`]; this module adds format detection and whole-file
//! detect/clean entry points so a Rust caller does not have to re-implement the
//! dispatch the CLI uses.
//!
//! ```no_run
//! use std::path::Path;
//! use scrub::{clean_file_in_place, detect_file, CleanOptions};
//!
//! // Detect first (works on text files and EPUBs alike).
//! let report = detect_file(Path::new("book.epub"))?;
//! if !report.is_clean() {
//!     // Clean in place, returning per-file statistics.
//!     let stats = clean_file_in_place(Path::new("book.epub"), CleanOptions::default())?;
//!     println!("removed {} carrier(s)", stats.removed());
//! }
//! # Ok::<(), anyhow::Error>(())
//! ```

use crate::epub::{clean_epub, inspect_epub, EpubCleanStats, EpubInspectReport};
use crate::unicode::{clean_text, inspect_text, CleanOptions, CleanStats, InspectReport};
use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};

/// The kind of input a path is handled as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// UTF-8 text (markdown, HTML, XML, plain text, …).
    Text,
    /// EPUB (ZIP) container.
    Epub,
}

/// Classify a path by extension. `.epub` is an EPUB; everything else is treated
/// as UTF-8 text.
pub fn kind_of(path: &Path) -> Kind {
    if path
        .extension()
        .map(|e| e.eq_ignore_ascii_case("epub"))
        .unwrap_or(false)
    {
        Kind::Epub
    } else {
        Kind::Text
    }
}

/// Result of detecting carriers/metadata in a file.
#[derive(serde::Serialize)]
#[serde(untagged)]
pub enum DetectReport {
    Text(InspectReport),
    Epub(EpubInspectReport),
}

impl DetectReport {
    /// True when nothing suspicious (and, for EPUBs, no stray generator meta)
    /// was found.
    pub fn is_clean(&self) -> bool {
        match self {
            DetectReport::Text(r) => r.suspicious_total == 0,
            DetectReport::Epub(r) => r.suspicious_total == 0 && r.generator_meta_total == 0,
        }
    }

    /// Count of suspicious code points found.
    pub fn suspicious_total(&self) -> usize {
        match self {
            DetectReport::Text(r) => r.suspicious_total,
            DetectReport::Epub(r) => r.suspicious_total,
        }
    }
}

/// Statistics from cleaning a file.
#[derive(serde::Serialize)]
#[serde(untagged)]
pub enum CleanReport {
    Text(CleanStats),
    Epub(EpubCleanStats),
}

impl CleanReport {
    /// Number of carriers stripped.
    pub fn removed(&self) -> usize {
        match self {
            CleanReport::Text(s) => s.removed_count,
            CleanReport::Epub(s) => s.removed_chars,
        }
    }

    /// Number of code points replaced (e.g. space homoglyphs folded to `U+0020`).
    pub fn replaced(&self) -> usize {
        match self {
            CleanReport::Text(s) => s.replaced_count,
            CleanReport::Epub(s) => s.replaced_chars,
        }
    }

    /// Whether cleaning changed anything.
    pub fn changed(&self) -> bool {
        match self {
            CleanReport::Text(s) => s.removed_count > 0 || s.replaced_count > 0 || s.nfkc_changed,
            CleanReport::Epub(s) => {
                s.removed_chars > 0 || s.replaced_chars > 0 || s.generator_meta_removed > 0
            }
        }
    }
}

fn read_utf8(path: &Path) -> Result<String> {
    let raw = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    String::from_utf8(raw).with_context(|| format!("{} is not valid UTF-8 text", path.display()))
}

/// Detect carriers/metadata in `path` without modifying it.
pub fn detect_file(path: &Path) -> Result<DetectReport> {
    if !path.is_file() {
        bail!("not a file: {}", path.display());
    }
    Ok(match kind_of(path) {
        Kind::Epub => DetectReport::Epub(inspect_epub(path)?),
        Kind::Text => DetectReport::Text(inspect_text(&read_utf8(path)?, false, false)),
    })
}

/// Clean `src`, writing the cleaned output to `dst` (which may equal `src`).
pub fn clean_file(src: &Path, dst: &Path, opts: CleanOptions) -> Result<CleanReport> {
    if !src.is_file() {
        bail!("not a file: {}", src.display());
    }
    match kind_of(src) {
        Kind::Epub => {
            // Clean to a temp file, then move into place (safe even if dst == src).
            let tmp: PathBuf = dst.with_extension("epub.scrub.tmp");
            let stats = clean_epub(src, &tmp, opts)?;
            std::fs::rename(&tmp, dst)
                .with_context(|| format!("moving cleaned EPUB into {}", dst.display()))?;
            Ok(CleanReport::Epub(stats))
        }
        Kind::Text => {
            let (cleaned, stats) = clean_text(&read_utf8(src)?, opts);
            std::fs::write(dst, cleaned.as_bytes())
                .with_context(|| format!("writing {}", dst.display()))?;
            Ok(CleanReport::Text(stats))
        }
    }
}

/// Clean `path` in place.
pub fn clean_file_in_place(path: &Path, opts: CleanOptions) -> Result<CleanReport> {
    clean_file(path, path, opts)
}

/// Default output path for a non-in-place clean: `<stem>.cleaned.<ext>`.
pub fn cleaned_path(path: &Path) -> PathBuf {
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("out");
    let name = match path.extension().and_then(|s| s.to_str()) {
        Some(e) => format!("{stem}.cleaned.{e}"),
        None => format!("{stem}.cleaned"),
    };
    path.with_file_name(name)
}
