//! Differential test: the Rust Layer A port must produce byte-identical output
//! to the vendored Python reference across a carrier/preservation corpus and
//! every option combination. Skips gracefully if `python3` is absent.

use scrub::unicode::{clean_text, CleanOptions};
use std::io::Write;
use std::process::{Command, Stdio};

fn python_clean(input: &str, flags: &[&str]) -> Option<Vec<u8>> {
    let script = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/reference/run_clean.py");
    let mut child = Command::new("python3")
        .arg(script)
        .args(flags)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .ok()?;
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .ok()?;
    let out = child.wait_with_output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(out.stdout)
}

fn opts_from(flags: &[&str]) -> CleanOptions {
    CleanOptions {
        nfkc: flags.contains(&"--nfkc"),
        aggressive_homoglyphs: flags.contains(&"--aggressive-homoglyphs"),
        normalize_spaces: !flags.contains(&"--no-normalize-spaces"),
        strip_emoji_glue: flags.contains(&"--strip-emoji-glue"),
        strip_bidi: flags.contains(&"--strip-bidi"),
    }
}

fn corpus() -> Vec<String> {
    vec![
        // Plain ASCII / prose.
        "The quick brown fox.".into(),
        "Español: acción, corazón, ¿qué tal?".into(),
        // Zero-width and format carriers embedded in words.
        "in\u{200B}visible zero width".into(),
        "word\u{200C}joiner\u{200D}here".into(),
        "bom\u{FEFF}mark and word\u{2060}joiner".into(),
        "soft\u{00AD}hyphen".into(),
        // Space homoglyphs (incl. Spanish no-break space usage).
        "N\u{00A0}1 and 10\u{202F}km and em\u{2003}space".into(),
        // Bidi controls: paired embedding (preserved) vs stray override (stripped).
        "abc\u{202A}def\u{202C}ghi".into(),
        "abc\u{202E}reversed".into(),
        "mark\u{200E}here\u{200F}too".into(),
        // Emoji: ZWJ sequences and variation selectors (must be preserved).
        "family \u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467} ok".into(),
        "heart on fire \u{2764}\u{FE0F}\u{200D}\u{1F525} done".into(),
        "scales \u{2696}\u{FE0F} of justice".into(),
        // Subdivision flag tag sequence (Scotland).
        "flag \u{1F3F4}\u{E0067}\u{E0062}\u{E0073}\u{E0063}\u{E0074}\u{E007F} end".into(),
        // Script joiners in-context (Persian) vs free-floating.
        "persian \u{0645}\u{06CC}\u{200C}\u{0631}\u{0648}\u{0645} word".into(),
        "free \u{200C} joiner".into(),
        // Tag chars / private use free-floating (stripped).
        "stray\u{E0041}tag and priv\u{E000}use".into(),
        // Fullwidth + Cyrillic confusables (only touched with --aggressive).
        "\u{FF21}\u{FF42}\u{FF43} and \u{0430}\u{0435}".into(),
        // Interlinear + other Cf.
        "note\u{FFF9}anchor\u{FFFB}".into(),
        // Empty and whitespace-only.
        "".into(),
        "   ".into(),
    ]
}

#[test]
fn rust_matches_python_reference() {
    let flag_sets: Vec<Vec<&str>> = vec![
        vec![],
        vec!["--nfkc"],
        vec!["--aggressive-homoglyphs"],
        vec!["--no-normalize-spaces"],
        vec!["--strip-emoji-glue"],
        vec!["--strip-bidi"],
        vec!["--strip-emoji-glue", "--strip-bidi"],
        vec!["--nfkc", "--aggressive-homoglyphs"],
    ];

    // Probe once; if python3 is unavailable, skip the whole differential test.
    if python_clean("probe", &[]).is_none() {
        eprintln!("SKIP: python3 reference not runnable");
        return;
    }

    let mut compared = 0;
    for input in corpus() {
        for flags in &flag_sets {
            let py = python_clean(&input, flags)
                .unwrap_or_else(|| panic!("python failed for {input:?} flags {flags:?}"));
            let (rust, _stats) = clean_text(&input, opts_from(flags));
            assert_eq!(
                rust.as_bytes(),
                py.as_slice(),
                "mismatch for input {input:?} flags {flags:?}\n rust={:?}\n  py ={:?}",
                rust,
                String::from_utf8_lossy(&py),
            );
            compared += 1;
        }
    }
    assert!(compared > 0);
    eprintln!("differential: {compared} (input, flags) pairs byte-identical");
}
