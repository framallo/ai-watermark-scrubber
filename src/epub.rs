//! EPUB container cleaning.
//!
//! An EPUB is a ZIP. We clean the text-typed entries (Layer A) and remove stray
//! `<meta name="generator">` tags from markup, passing every binary entry
//! (images, fonts) through byte-identical. The repack keeps `mimetype` as the
//! first entry, STORED (uncompressed), as the OCF spec requires.

use crate::unicode::{clean_text, inspect_text, CleanOptions};
use anyhow::{Context, Result};
use std::io::{Cursor, Read, Write};
use std::path::Path;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

/// Entries whose bytes are UTF-8 markup/text we may rewrite.
fn is_text_entry(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    [
        ".xhtml", ".html", ".htm", ".opf", ".ncx", ".xml", ".css", ".svg", ".txt", ".md",
    ]
    .iter()
    .any(|ext| lower.ends_with(ext))
}

/// Remove any `<meta ... name="generator" ...>` tag (self-closed or open) from
/// markup. Generic build-tool signatures are provenance breadcrumbs, not content.
fn strip_generator_meta(html: &str) -> (String, usize) {
    let mut out = String::with_capacity(html.len());
    let mut removed = 0;
    let bytes = html.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'<' {
            // Find the end of this tag.
            if let Some(rel_end) = html[i..].find('>') {
                let end = i + rel_end + 1;
                let tag = &html[i..end];
                let low = tag.to_ascii_lowercase();
                if low.starts_with("<meta")
                    && (low.contains("name=\"generator\"") || low.contains("name='generator'"))
                {
                    removed += 1;
                    // Swallow one trailing newline to avoid leaving a blank line.
                    i = end;
                    if i < bytes.len() && bytes[i] == b'\n' {
                        i += 1;
                    }
                    continue;
                }
            }
        }
        // Copy this char (handle multibyte by copying to next char boundary).
        let ch = html[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    (out, removed)
}

#[derive(Default, serde::Serialize)]
pub struct EpubCleanStats {
    pub entries_total: usize,
    pub entries_text: usize,
    pub removed_chars: usize,
    pub replaced_chars: usize,
    pub generator_meta_removed: usize,
}

/// Clean an EPUB from `src` to `dst`. Returns aggregate stats.
pub fn clean_epub(src: &Path, dst: &Path, opts: CleanOptions) -> Result<EpubCleanStats> {
    let data = std::fs::read(src).with_context(|| format!("reading {}", src.display()))?;
    let mut archive = ZipArchive::new(Cursor::new(data))
        .with_context(|| format!("{} is not a valid zip/EPUB", src.display()))?;

    let mut buf = Vec::new();
    let mut zw = ZipWriter::new(Cursor::new(&mut buf));
    let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    let mut stats = EpubCleanStats::default();

    // Preserve entry order but hoist `mimetype` to the front, STORED.
    let mut order: Vec<usize> = (0..archive.len()).collect();
    order.sort_by_key(|&i| {
        archive
            .by_index(i)
            .map(|f| if f.name() == "mimetype" { 0 } else { 1 })
            .unwrap_or(1)
    });

    for idx in order {
        let (name, is_dir, raw) = {
            let mut f = archive.by_index(idx)?;
            let name = f.name().to_string();
            let is_dir = f.is_dir();
            let mut raw = Vec::new();
            if !is_dir {
                f.read_to_end(&mut raw)?;
            }
            (name, is_dir, raw)
        };
        stats.entries_total += 1;

        if is_dir {
            zw.add_directory(&name, deflated)?;
            continue;
        }

        if name == "mimetype" {
            zw.start_file(&name, stored)?;
            zw.write_all(&raw)?;
            continue;
        }

        if is_text_entry(&name) {
            if let Ok(text) = std::str::from_utf8(&raw) {
                let (mut cleaned, cstats) = clean_text(text, opts);
                let gen_removed = if name.ends_with(".opf")
                    || name.ends_with(".xhtml")
                    || name.ends_with(".html")
                    || name.ends_with(".htm")
                {
                    let (c, n) = strip_generator_meta(&cleaned);
                    cleaned = c;
                    n
                } else {
                    0
                };
                stats.entries_text += 1;
                stats.removed_chars += cstats.removed_count;
                stats.replaced_chars += cstats.replaced_count;
                stats.generator_meta_removed += gen_removed;
                zw.start_file(&name, deflated)?;
                zw.write_all(cleaned.as_bytes())?;
                continue;
            }
        }

        // Binary or non-UTF-8: pass through byte-identical.
        zw.start_file(&name, deflated)?;
        zw.write_all(&raw)?;
    }

    zw.finish()?;
    std::fs::write(dst, &buf).with_context(|| format!("writing {}", dst.display()))?;
    Ok(stats)
}

#[derive(serde::Serialize)]
pub struct EpubInspectEntry {
    pub entry: String,
    pub suspicious_total: usize,
    pub generator_meta: usize,
    pub hits: Vec<crate::unicode::CharHit>,
}

#[derive(serde::Serialize)]
pub struct EpubInspectReport {
    pub entries_scanned: usize,
    pub suspicious_total: usize,
    pub generator_meta_total: usize,
    pub findings: Vec<EpubInspectEntry>,
}

/// Inspect an EPUB's text entries for Layer A carriers and generator metadata.
pub fn inspect_epub(src: &Path) -> Result<EpubInspectReport> {
    let data = std::fs::read(src).with_context(|| format!("reading {}", src.display()))?;
    let mut archive = ZipArchive::new(Cursor::new(data))
        .with_context(|| format!("{} is not a valid zip/EPUB", src.display()))?;

    let mut report = EpubInspectReport {
        entries_scanned: 0,
        suspicious_total: 0,
        generator_meta_total: 0,
        findings: Vec::new(),
    };

    for idx in 0..archive.len() {
        let mut f = archive.by_index(idx)?;
        if f.is_dir() || !is_text_entry(f.name()) {
            continue;
        }
        let name = f.name().to_string();
        let mut raw = Vec::new();
        f.read_to_end(&mut raw)?;
        let Ok(text) = std::str::from_utf8(&raw) else {
            continue;
        };
        report.entries_scanned += 1;
        let rep = inspect_text(text, false, false);
        let (_c, gen) = strip_generator_meta(text);
        if rep.suspicious_total == 0 && gen == 0 {
            continue;
        }
        report.suspicious_total += rep.suspicious_total;
        report.generator_meta_total += gen;
        report.findings.push(EpubInspectEntry {
            entry: name,
            suspicious_total: rep.suspicious_total,
            generator_meta: gen,
            hits: rep.hits,
        });
    }

    Ok(report)
}
