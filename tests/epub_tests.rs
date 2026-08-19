//! EPUB container tests: injected carriers/metadata are removed, binary entries
//! pass through byte-identical, and the repack stays OCF-valid (mimetype first
//! and STORED). Runs epubcheck too when it is installed.

mod common;
use common::{build_dirty_epub, PNG_BYTES};
use scrub::epub::{clean_epub, inspect_epub};
use scrub::unicode::CleanOptions;
use std::io::{Cursor, Read};
use std::path::Path;
use zip::{CompressionMethod, ZipArchive};

fn read_entry(path: &Path, name: &str) -> Vec<u8> {
    let data = std::fs::read(path).unwrap();
    let mut archive = ZipArchive::new(Cursor::new(data)).unwrap();
    let mut f = archive.by_name(name).unwrap();
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).unwrap();
    buf
}

#[test]
fn epub_clean_removes_carriers_and_metadata_preserves_binary() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("book.epub");
    let dst = dir.path().join("book.cleaned.epub");
    build_dirty_epub(&src);

    // Detect BEFORE: carriers + generator meta present.
    let before = inspect_epub(&src).unwrap();
    assert!(before.suspicious_total > 0, "fixture should have carriers");
    assert!(before.generator_meta_total >= 2, "fixture should have generator meta");

    let stats = clean_epub(&src, &dst, CleanOptions::default()).unwrap();
    assert!(stats.removed_chars > 0);
    assert!(stats.generator_meta_removed >= 2);

    // Detect AFTER: fully clean.
    let after = inspect_epub(&dst).unwrap();
    assert_eq!(after.suspicious_total, 0, "no carriers may remain: {:?}", after.findings.iter().map(|f| &f.entry).collect::<Vec<_>>());
    assert_eq!(after.generator_meta_total, 0, "no generator meta may remain");

    // Zero-width / nbsp gone from chapter; emoji ZWJ family preserved.
    let ch = String::from_utf8(read_entry(&dst, "OEBPS/chapter-001.xhtml")).unwrap();
    assert!(!ch.contains('\u{200B}') && !ch.contains('\u{FEFF}') && !ch.contains('\u{00A0}'));
    assert!(ch.contains("\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}"), "emoji family must survive");

    // OPF no longer carries the generator tag.
    let opf = String::from_utf8(read_entry(&dst, "OEBPS/content.opf")).unwrap();
    assert!(!opf.to_lowercase().contains("name=\"generator\""));
    assert!(opf.contains("<dc:title>Test Book</dc:title>"), "nbsp folded in title");

    // Binary image is byte-identical.
    assert_eq!(read_entry(&dst, "OEBPS/img.png"), PNG_BYTES);
}

#[test]
fn epub_repack_is_ocf_valid_shape() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("book.epub");
    let dst = dir.path().join("out.epub");
    build_dirty_epub(&src);
    clean_epub(&src, &dst, CleanOptions::default()).unwrap();

    let data = std::fs::read(&dst).unwrap();
    let mut archive = ZipArchive::new(Cursor::new(data)).unwrap();
    // First entry must be `mimetype` and STORED.
    let first = archive.by_index(0).unwrap();
    assert_eq!(first.name(), "mimetype");
    assert_eq!(first.compression(), CompressionMethod::Stored);
    drop(first);
    let mut mt = archive.by_name("mimetype").unwrap();
    let mut s = String::new();
    mt.read_to_string(&mut s).unwrap();
    assert_eq!(s, "application/epub+zip");
}

#[test]
fn epub_passes_epubcheck_if_available() {
    // Only meaningful if epubcheck is installed; skip otherwise.
    let probe = std::process::Command::new("epubcheck").arg("--version").output();
    if probe.is_err() {
        eprintln!("SKIP: epubcheck not installed");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("book.epub");
    let dst = dir.path().join("out.epub");
    build_dirty_epub(&src);
    clean_epub(&src, &dst, CleanOptions::default()).unwrap();

    let out = std::process::Command::new("epubcheck").arg(&dst).output().unwrap();
    assert!(
        out.status.success(),
        "epubcheck failed:\n{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
}
