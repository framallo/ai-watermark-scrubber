//! Behavioral unit tests for the Layer A cleaner/detector.

use scrub::unicode::{clean_text, inspect_text, CleanOptions};

fn clean_default(s: &str) -> String {
    clean_text(s, CleanOptions::default()).0
}

#[test]
fn strips_zero_width_and_format_carriers() {
    // ZWSP, ZWNJ(free), ZWJ(free), word joiner, BOM, soft hyphen.
    let dirty = "a\u{200B}b\u{2060}c\u{FEFF}d\u{00AD}e";
    assert_eq!(clean_default(dirty), "abcde");
}

#[test]
fn folds_space_homoglyphs_to_ascii_space() {
    let dirty = "x\u{00A0}y\u{202F}z\u{2003}w"; // nbsp, narrow nbsp, em space
    assert_eq!(clean_default(dirty), "x y z w");
    // With --no-normalize-spaces the exotic spaces survive.
    let opts = CleanOptions {
        normalize_spaces: false,
        ..CleanOptions::default()
    };
    assert_eq!(clean_text(dirty, opts).0, dirty);
}

#[test]
fn preserves_emoji_zwj_sequences() {
    let family = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}";
    assert_eq!(clean_default(family), family, "ZWJ family must survive");
    let heart_fire = "\u{2764}\u{FE0F}\u{200D}\u{1F525}";
    assert_eq!(clean_default(heart_fire), heart_fire);
}

#[test]
fn preserves_paired_bidi_but_strips_stray_override() {
    // Paired LRE ... PDF is legitimate embedding -> preserved.
    let paired = "a\u{202A}b\u{202C}c";
    assert_eq!(clean_default(paired), paired);
    // Lone RLO override reorders unrelated text -> stripped by default.
    let stray = "a\u{202E}b";
    assert_eq!(clean_default(stray), "ab");
}

#[test]
fn aggressive_maps_confusables_only_when_requested() {
    let dirty = "\u{FF21}\u{0430}"; // fullwidth A, cyrillic a
    assert_eq!(clean_default(dirty), dirty, "confusables untouched by default");
    let opts = CleanOptions {
        aggressive_homoglyphs: true,
        ..CleanOptions::default()
    };
    assert_eq!(clean_text(dirty, opts).0, "Aa");
}

#[test]
fn detect_flags_carriers_and_reports_clean() {
    let rep = inspect_text("a\u{200B}b\u{00A0}c", false, false);
    assert!(rep.suspicious_total >= 2);
    // Zero-width is "probable"; space homoglyph is "informational".
    assert!(rep.hits.iter().any(|h| h.codepoint == "U+200B" && h.confidence == "probable"));
    assert!(rep.hits.iter().any(|h| h.codepoint == "U+00A0" && h.confidence == "informational"));

    let clean = inspect_text("perfectly normal text", false, false);
    assert_eq!(clean.suspicious_total, 0);
    assert!(clean.hits.is_empty());
}

#[test]
fn cleaning_makes_detection_clean() {
    let dirty = "in\u{200B}visible\u{FEFF} text\u{00A0}here\u{202E}!";
    let (cleaned, stats) = clean_text(dirty, CleanOptions::default());
    assert!(stats.removed_count > 0);
    let rep = inspect_text(&cleaned, false, false);
    assert_eq!(rep.suspicious_total, 0, "cleaned text must have no carriers left");
}

#[test]
fn idempotent() {
    let dirty = "a\u{200B}b\u{00A0}c\u{1F468}\u{200D}\u{1F467}";
    let once = clean_default(dirty);
    let twice = clean_default(&once);
    assert_eq!(once, twice);
}
