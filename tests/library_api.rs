//! Exercise the high-level library API the way an embedding Rust program would.

mod common;
use common::build_dirty_epub;
use scrub::{clean_file, clean_file_in_place, detect_file, CleanOptions, Kind};

#[test]
fn detect_and_clean_text_file_via_library() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("draft.md");
    std::fs::write(&f, "in\u{200B}visible\u{00A0}mark and \u{202E}override").unwrap();

    let before = detect_file(&f).unwrap();
    assert!(!before.is_clean());
    assert!(before.suspicious_total() >= 2);

    let report = clean_file_in_place(&f, CleanOptions::default()).unwrap();
    assert!(report.changed());
    assert!(report.removed() >= 1);

    let after = detect_file(&f).unwrap();
    assert!(after.is_clean());
    assert_eq!(std::fs::read_to_string(&f).unwrap(), "invisible mark and override");
}

#[test]
fn detect_and_clean_epub_via_library() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("book.epub");
    let dst = dir.path().join("book.cleaned.epub");
    build_dirty_epub(&src);

    assert_eq!(scrub::kind_of(&src), Kind::Epub);

    let before = detect_file(&src).unwrap();
    assert!(!before.is_clean());

    let report = clean_file(&src, &dst, CleanOptions::default()).unwrap();
    assert!(report.changed());

    let after = detect_file(&dst).unwrap();
    assert!(after.is_clean(), "cleaned EPUB should detect clean");
}
