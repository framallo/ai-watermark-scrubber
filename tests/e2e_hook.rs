//! End-to-end: exercise the exact CLI contract the bookmill post-build hook
//! relies on. The hook runs `scrub clean --in-place {file}` against a freshly
//! built EPUB; here we build a watermarked EPUB, run the real binary the same
//! way, and prove the artifact comes out clean (and still epubcheck-valid).

mod common;
use common::build_dirty_epub;
use std::path::Path;
use std::process::Command;

fn scrub_bin() -> &'static str {
    env!("CARGO_BIN_EXE_scrub")
}

fn run(args: &[&str]) -> std::process::Output {
    Command::new(scrub_bin())
        .args(args)
        .output()
        .expect("failed to run scrub binary")
}

#[test]
fn hook_invocation_cleans_built_epub_end_to_end() {
    let dir = tempfile::tempdir().unwrap();
    let epub = dir.path().join("open-the-valve.epub");
    build_dirty_epub(&epub);
    let path = epub.to_str().unwrap();

    // 1. `scrub detect` must FAIL (exit 1) on the watermarked artifact — this is
    //    the signal a gating hook would use.
    let before = run(&["detect", path]);
    assert!(
        !before.status.success(),
        "detect should exit non-zero on a dirty EPUB"
    );

    // 2. The hook's actual command: `scrub clean --in-place {file}`. Exit 0.
    let cleaned = run(&["clean", "--in-place", path]);
    assert!(
        cleaned.status.success(),
        "clean --in-place failed: {}",
        String::from_utf8_lossy(&cleaned.stderr)
    );

    // 3. `scrub detect` now SUCCEEDS (exit 0): the artifact is clean.
    let after = run(&["detect", path]);
    assert!(
        after.status.success(),
        "detect should exit zero after cleaning; stderr:\n{}",
        String::from_utf8_lossy(&after.stderr)
    );

    // 4. The cleaned-in-place EPUB is still epubcheck-valid (when available).
    assert_epub_valid_if_epubcheck(&epub);
}

fn assert_epub_valid_if_epubcheck(epub: &Path) {
    if Command::new("epubcheck").arg("--version").output().is_err() {
        eprintln!("SKIP: epubcheck not installed");
        return;
    }
    let out = Command::new("epubcheck").arg(epub).output().unwrap();
    assert!(
        out.status.success(),
        "cleaned EPUB failed epubcheck:\n{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
}
