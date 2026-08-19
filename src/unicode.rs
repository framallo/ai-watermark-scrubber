//! Layer A: invisible / format Unicode detection and cleaning.
//!
//! Ported from the reference Python implementation (`text_unicode.py`). The
//! design goal is behavioral parity on the deterministic edit-based carriers:
//! zero-width and format controls are stripped, space homoglyphs are folded to
//! U+0020, and load-bearing invisibles (emoji glue, script joiners, paired bidi
//! embeddings, same-script fillers, orthographic Cf marks) are preserved.

use std::collections::BTreeMap;
use unicode_general_category::{get_general_category, GeneralCategory};
use unicode_normalization::UnicodeNormalization;

// ---------- codepoint tables ----------

/// Format / invisible controls commonly used for steganography or broken pastes.
fn is_strip_codepoint_table(cp: u32) -> bool {
    matches!(
        cp,
        0x00AD | 0x034F | 0x061C | 0x115F | 0x1160 | 0x17B4 | 0x17B5
            | 0x180B | 0x180C | 0x180D | 0x180E
            | 0x200B | 0x200C | 0x200D | 0x200E | 0x200F
            | 0x202A | 0x202B | 0x202C | 0x202D | 0x202E
            | 0x2060 | 0x2061 | 0x2062 | 0x2063 | 0x2064
            | 0x2066 | 0x2067 | 0x2068 | 0x2069
            | 0x206A | 0x206B | 0x206C | 0x206D | 0x206E | 0x206F
            | 0xFEFF
            | 0xFE00 | 0xFE01 | 0xFE02 | 0xFE03 | 0xFE04 | 0xFE05 | 0xFE06 | 0xFE07
            | 0xFE08 | 0xFE09 | 0xFE0A | 0xFE0B | 0xFE0C | 0xFE0D | 0xFE0E | 0xFE0F
            | 0xFFF9 | 0xFFFA | 0xFFFB
    )
}

/// Spaces that look like (or substitute for) U+0020.
fn space_homoglyph(cp: u32) -> Option<char> {
    matches!(
        cp,
        0x00A0 | 0x1680 | 0x2000 | 0x2001 | 0x2002 | 0x2003 | 0x2004 | 0x2005
            | 0x2006 | 0x2007 | 0x2008 | 0x2009 | 0x200A | 0x202F | 0x205F | 0x3000
    )
    .then_some(' ')
}

/// Optional confusable Latin lookalikes (aggressive mode only).
fn latin_confusable(cp: u32) -> Option<char> {
    let c = match cp {
        0x0410 => 'A',
        0x0412 => 'B',
        0x0415 => 'E',
        0x041A => 'K',
        0x041C => 'M',
        0x041D => 'H',
        0x041E => 'O',
        0x0420 => 'P',
        0x0421 => 'C',
        0x0422 => 'T',
        0x0425 => 'X',
        0x0430 => 'a',
        0x0435 => 'e',
        0x043E => 'o',
        0x0440 => 'p',
        0x0441 => 'c',
        0x0443 => 'y',
        0x0445 => 'x',
        0x0456 => 'i',
        0xFF21..=0xFF3A => char::from_u32(cp - 0xFF21 + b'A' as u32)?,
        0xFF41..=0xFF5A => char::from_u32(cp - 0xFF41 + b'a' as u32)?,
        _ => return None,
    };
    Some(c)
}

const VS_SUPPLEMENT: std::ops::RangeInclusive<u32> = 0xE0100..=0xE01EF;

fn is_private_use(cp: u32) -> bool {
    (0xE000..=0xF8FF).contains(&cp)
        || (0xF0000..=0xFFFFD).contains(&cp)
        || (0x100000..=0x10FFFD).contains(&cp)
}

fn is_strip_cp(cp: u32) -> bool {
    if is_strip_codepoint_table(cp) {
        return true;
    }
    if VS_SUPPLEMENT.contains(&cp) {
        return true;
    }
    if (0xE0001..=0xE007F).contains(&cp) {
        return true;
    }
    is_private_use(cp)
}

fn is_bidi_cp(cp: u32) -> bool {
    matches!(
        cp,
        0x061C | 0x200E | 0x200F | 0x202A | 0x202B | 0x202C | 0x202D | 0x202E
            | 0x2066 | 0x2067 | 0x2068 | 0x2069
    )
}

fn is_preservable_bidi(cp: u32) -> bool {
    matches!(cp, 0x061C | 0x200E | 0x200F | 0x2066 | 0x2067 | 0x2068 | 0x2069)
}

fn is_zw_family(cp: u32) -> bool {
    matches!(cp, 0x200B | 0x200C | 0x200D | 0x2060 | 0xFEFF | 0x180E)
}

fn is_emoji_glue(cp: u32) -> bool {
    matches!(cp, 0x200D | 0xFE0E | 0xFE0F)
}

fn is_emoji_base(cp: u32) -> bool {
    if (0x1F000..=0x1FAFF).contains(&cp)
        || (0x2190..=0x25FF).contains(&cp)
        || (0x2600..=0x27BF).contains(&cp)
        || (0x2B00..=0x2BFF).contains(&cp)
    {
        return true;
    }
    if matches!(cp, 0x00A9 | 0x00AE | 0x2122 | 0x3030 | 0x303D | 0x3297 | 0x3299) {
        return true;
    }
    matches!(cp, 0x0023 | 0x002A) || (0x0030..=0x0039).contains(&cp)
}

const SCRIPT_JOINERS: [u32; 2] = [0x200C, 0x200D];
fn is_script_joiner(cp: u32) -> bool {
    SCRIPT_JOINERS.contains(&cp)
}
fn is_tag_range(cp: u32) -> bool {
    (0xE0020..=0xE007F).contains(&cp)
}
fn is_orthographic_cf(cp: u32) -> bool {
    matches!(
        cp,
        0x0600 | 0x0601 | 0x0602 | 0x0603 | 0x0604 | 0x0605 | 0x06DD | 0x070F
            | 0x08E2 | 0x110BD | 0x110CD
    )
}
fn is_mongolian_fvs(cp: u32) -> bool {
    matches!(cp, 0x180B | 0x180C | 0x180D)
}
fn is_khmer_vowel(cp: u32) -> bool {
    matches!(cp, 0x17B4 | 0x17B5)
}
fn is_hangul_filler(cp: u32) -> bool {
    matches!(cp, 0x115F | 0x1160)
}

fn is_variation_selector(cp: u32) -> bool {
    VS_SUPPLEMENT.contains(&cp) || (0xFE00..=0xFE0F).contains(&cp) || (0x180B..=0x180D).contains(&cp)
}

// ---------- general-category helpers ----------

fn cat_first_is_letter_or_mark(cp: u32) -> bool {
    let Some(c) = char::from_u32(cp) else {
        return false;
    };
    matches!(
        get_general_category(c),
        GeneralCategory::UppercaseLetter
            | GeneralCategory::LowercaseLetter
            | GeneralCategory::TitlecaseLetter
            | GeneralCategory::ModifierLetter
            | GeneralCategory::OtherLetter
            | GeneralCategory::NonspacingMark
            | GeneralCategory::SpacingMark
            | GeneralCategory::EnclosingMark
    )
}

fn cat_first_is_letter(cp: u32) -> bool {
    let Some(c) = char::from_u32(cp) else {
        return false;
    };
    matches!(
        get_general_category(c),
        GeneralCategory::UppercaseLetter
            | GeneralCategory::LowercaseLetter
            | GeneralCategory::TitlecaseLetter
            | GeneralCategory::ModifierLetter
            | GeneralCategory::OtherLetter
    )
}

fn cat_is_format(cp: u32) -> bool {
    let Some(c) = char::from_u32(cp) else {
        return false;
    };
    get_general_category(c) == GeneralCategory::Format
}

fn is_cjk_ideograph(cp: u32) -> bool {
    (0x3400..=0x4DBF).contains(&cp)
        || (0x4E00..=0x9FFF).contains(&cp)
        || (0xF900..=0xFAFF).contains(&cp)
        || (0x20000..=0x323AF).contains(&cp)
}
fn is_mongolian_base(cp: u32) -> bool {
    (0x1800..=0x18AF).contains(&cp)
}
fn is_mongolian_letter(cp: u32) -> bool {
    (0x1800..=0x18AF).contains(&cp) && cat_first_is_letter(cp)
}
fn is_khmer_letter(cp: u32) -> bool {
    (0x1780..=0x17FF).contains(&cp) && cat_first_is_letter(cp)
}
fn is_hangul_jamo(cp: u32) -> bool {
    (0x1100..=0x11FF).contains(&cp)
        || (0xA960..=0xA97C).contains(&cp)
        || (0xD7B0..=0xD7C6).contains(&cp)
}

/// Broad script group where ZWJ/ZWNJ can be orthographic. `None` when the
/// character is not a letter/mark in one of those blocks.
fn joining_script(cp: u32) -> Option<&'static str> {
    for (start, end, name) in [
        (0x0600u32, 0x08FFu32, "arabic"),
        (0x0900, 0x0DFF, "indic"),
        (0x0F00, 0x109F, "south-asian"),
        (0x1780, 0x17FF, "khmer"),
        (0x1800, 0x18AF, "mongolian"),
    ] {
        if (start..=end).contains(&cp) && cat_first_is_letter_or_mark(cp) {
            return Some(name);
        }
    }
    None
}

/// Load-bearing invisible: emoji glue, variation selector, script joiner, flag
/// tag char, or same-script filler/selector. These do not advance `prev_kept`.
fn is_glue(cp: u32) -> bool {
    is_emoji_glue(cp)
        || is_variation_selector(cp)
        || is_script_joiner(cp)
        || is_tag_range(cp)
        || is_mongolian_fvs(cp)
        || is_khmer_vowel(cp)
        || is_hangul_filler(cp)
}

// ---------- structural index sets ----------

/// Indices belonging to complete subdivision-flag tag sequences.
fn valid_flag_tag_indices(chars: &[char]) -> std::collections::HashSet<usize> {
    let mut valid = std::collections::HashSet::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] as u32 != 0x1F3F4 {
            i += 1;
            continue;
        }
        let mut j = i + 1;
        while j < chars.len() && (0xE0020..=0xE007E).contains(&(chars[j] as u32)) {
            j += 1;
        }
        if j > i + 1 && j < chars.len() && chars[j] as u32 == 0xE007F {
            for k in (i + 1)..=j {
                valid.insert(k);
            }
            i = j + 1;
        } else {
            i += 1;
        }
    }
    valid
}

/// Indices belonging to complete LRE/RLE ... PDF pairs, excluding overrides.
fn valid_bidi_embedding_indices(chars: &[char]) -> std::collections::HashSet<usize> {
    let mut valid = std::collections::HashSet::new();
    let mut stack: Vec<(u32, usize)> = Vec::new();
    for (index, &ch) in chars.iter().enumerate() {
        let cp = ch as u32;
        if matches!(cp, 0x202A | 0x202B | 0x202D | 0x202E) {
            stack.push((cp, index));
        } else if cp == 0x202C {
            if let Some((opener, opener_index)) = stack.pop() {
                if matches!(opener, 0x202A | 0x202B) {
                    valid.insert(opener_index);
                    valid.insert(index);
                }
            }
        }
    }
    valid
}

// ---------- per-char decision ----------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Action {
    Keep,
    Strip,
    Replace,
}

#[derive(Clone, Copy)]
pub struct CleanOptions {
    pub nfkc: bool,
    pub aggressive_homoglyphs: bool,
    pub normalize_spaces: bool,
    pub strip_emoji_glue: bool,
    pub strip_bidi: bool,
}

impl Default for CleanOptions {
    fn default() -> Self {
        CleanOptions {
            nfkc: false,
            aggressive_homoglyphs: false,
            normalize_spaces: true,
            strip_emoji_glue: false,
            strip_bidi: false,
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn decide(
    ch: char,
    prev_kept: Option<char>,
    prev_input: Option<char>,
    next_input: Option<char>,
    valid_flag_tag: bool,
    valid_bidi_embedding: bool,
    normalize_spaces: bool,
    treat_confusables: bool,
    strip_emoji_glue: bool,
    strip_bidi: bool,
) -> (Action, char, Option<&'static str>) {
    let cp = ch as u32;
    if valid_bidi_embedding && !strip_bidi {
        return (Action::Keep, ch, None);
    }
    if is_preservable_bidi(cp) && !strip_bidi {
        return (Action::Keep, ch, None);
    }
    if let Some(prev) = prev_input {
        if !strip_emoji_glue {
            let prev_cp = prev as u32;
            if VS_SUPPLEMENT.contains(&cp) && is_cjk_ideograph(prev_cp) {
                return (Action::Keep, ch, None);
            }
            if (0x180B..=0x180D).contains(&cp) && is_mongolian_base(prev_cp) {
                return (Action::Keep, ch, None);
            }
            if (0xFE00..=0xFE0D).contains(&cp) && is_cjk_ideograph(prev_cp) {
                return (Action::Keep, ch, None);
            }
        }
    }
    if is_emoji_glue(cp) && !strip_emoji_glue {
        if matches!(cp, 0xFE0E | 0xFE0F) {
            if let Some(prev) = prev_input {
                if is_emoji_base(prev as u32) {
                    return (Action::Keep, ch, None);
                }
            }
        }
        if cp == 0x200D {
            if let (Some(pk), Some(ni)) = (prev_kept, next_input) {
                if is_emoji_base(pk as u32) && is_emoji_base(ni as u32) {
                    return (Action::Keep, ch, None);
                }
            }
        }
    }
    if !strip_emoji_glue {
        if is_script_joiner(cp) {
            if let (Some(prev), Some(next)) = (prev_input, next_input) {
                let ps = joining_script(prev as u32);
                let ns = joining_script(next as u32);
                if ps.is_some() && ps == ns {
                    return (Action::Keep, ch, None);
                }
            }
        }
        if is_tag_range(cp) && valid_flag_tag {
            return (Action::Keep, ch, None);
        }
        if is_mongolian_fvs(cp) {
            if let Some(pk) = prev_kept {
                if is_mongolian_letter(pk as u32) {
                    return (Action::Keep, ch, None);
                }
            }
        }
        if is_khmer_vowel(cp) {
            if let Some(pk) = prev_kept {
                if is_khmer_letter(pk as u32) {
                    return (Action::Keep, ch, None);
                }
            }
        }
        if is_hangul_filler(cp) {
            if let Some(pk) = prev_kept {
                if is_hangul_jamo(pk as u32) {
                    return (Action::Keep, ch, None);
                }
            }
        }
        if is_orthographic_cf(cp) {
            return (Action::Keep, ch, None);
        }
    }
    if is_strip_cp(cp) {
        return (Action::Strip, '\0', Some(strip_kind(cp)));
    }
    if normalize_spaces {
        if let Some(r) = space_homoglyph(cp) {
            return (Action::Replace, r, Some("space"));
        }
    }
    if treat_confusables {
        if let Some(r) = latin_confusable(cp) {
            return (Action::Replace, r, Some("confusable"));
        }
    }
    if cat_is_format(cp) && space_homoglyph(cp).is_none() {
        return (Action::Strip, '\0', Some("other_cf"));
    }
    (Action::Keep, ch, None)
}

fn strip_kind(cp: u32) -> &'static str {
    if (0xE0001..=0xE007F).contains(&cp) {
        return "tag_chars";
    }
    if VS_SUPPLEMENT.contains(&cp) || (0xFE00..=0xFE0F).contains(&cp) || (0x180B..=0x180D).contains(&cp)
    {
        return "variation_selector";
    }
    if is_bidi_cp(cp) {
        return "bidi";
    }
    if is_zw_family(cp) {
        return "zwj_family";
    }
    if is_private_use(cp) {
        return "private_use";
    }
    "strip"
}

// ---------- public API ----------

#[derive(Default, serde::Serialize)]
pub struct CleanStats {
    pub input_length: usize,
    pub output_length: usize,
    pub removed_count: usize,
    pub replaced_count: usize,
    pub nfkc_changed: bool,
    pub removed: BTreeMap<String, usize>,
    pub replaced: BTreeMap<String, usize>,
}

fn char_label(ch: char) -> String {
    let cp = ch as u32;
    let cat = char::from_u32(cp)
        .map(|c| format!("{:?}", get_general_category(c)))
        .unwrap_or_default();
    format!("U+{cp:04X} ({cat})")
}

/// Clean text: strip invisible/format carriers, fold space homoglyphs.
pub fn clean_text(text: &str, opts: CleanOptions) -> (String, CleanStats) {
    let chars: Vec<char> = text.chars().collect();
    let flag_tags = valid_flag_tag_indices(&chars);
    let bidi_pairs = valid_bidi_embedding_indices(&chars);

    let mut out = String::with_capacity(text.len());
    let mut stats = CleanStats {
        input_length: chars.len(),
        ..Default::default()
    };
    let mut prev_kept: Option<char> = None;

    for i in 0..chars.len() {
        let ch = chars[i];
        let prev_input = if i > 0 { Some(chars[i - 1]) } else { None };
        let next_input = chars.get(i + 1).copied();
        let (action, out_char, _kind) = decide(
            ch,
            prev_kept,
            prev_input,
            next_input,
            flag_tags.contains(&i),
            bidi_pairs.contains(&i),
            opts.normalize_spaces,
            opts.aggressive_homoglyphs,
            opts.strip_emoji_glue,
            opts.strip_bidi,
        );
        match action {
            Action::Keep => {
                out.push(out_char);
                if !is_glue(ch as u32) {
                    prev_kept = Some(out_char);
                }
            }
            Action::Replace => {
                out.push(out_char);
                *stats.replaced.entry(char_label(ch)).or_insert(0) += 1;
                stats.replaced_count += 1;
                prev_kept = Some(out_char);
            }
            Action::Strip => {
                *stats.removed.entry(char_label(ch)).or_insert(0) += 1;
                stats.removed_count += 1;
            }
        }
    }

    if opts.nfkc {
        let normalized: String = out.nfkc().collect();
        if normalized != out {
            stats.nfkc_changed = true;
            out = normalized;
        }
    }

    stats.output_length = out.chars().count();
    (out, stats)
}

#[derive(serde::Serialize)]
pub struct CharHit {
    pub codepoint: String,
    pub count: usize,
    pub kind: String,
    pub confidence: String,
    pub sample_offsets: Vec<usize>,
}

#[derive(serde::Serialize)]
pub struct InspectReport {
    pub length: usize,
    pub suspicious_total: usize,
    pub hits: Vec<CharHit>,
}

fn hit_confidence(kind: &str) -> &'static str {
    if kind == "space" {
        "informational"
    } else {
        "probable"
    }
}

/// Inspect text for Layer A carriers without modifying it.
pub fn inspect_text(text: &str, aggressive: bool, strip_emoji_glue: bool) -> InspectReport {
    let chars: Vec<char> = text.chars().collect();
    let flag_tags = valid_flag_tag_indices(&chars);
    let bidi_pairs = valid_bidi_embedding_indices(&chars);
    // key -> (cp, kind, offsets); ordered for stable output.
    let mut buckets: BTreeMap<(u32, String), Vec<usize>> = BTreeMap::new();
    let mut prev_kept: Option<char> = None;

    for i in 0..chars.len() {
        let ch = chars[i];
        let prev_input = if i > 0 { Some(chars[i - 1]) } else { None };
        let next_input = chars.get(i + 1).copied();
        // inspect uses normalize_spaces=true, strip_bidi=true (report bidi).
        let (action, out_char, kind) = decide(
            ch,
            prev_kept,
            prev_input,
            next_input,
            flag_tags.contains(&i),
            bidi_pairs.contains(&i),
            true,
            aggressive,
            strip_emoji_glue,
            true,
        );
        match kind {
            None => {
                if !is_glue(ch as u32) {
                    prev_kept = Some(out_char);
                }
            }
            Some(k) => {
                buckets.entry((ch as u32, k.to_string())).or_default().push(i);
                if action == Action::Replace {
                    prev_kept = Some(out_char);
                }
            }
        }
    }

    let mut hits: Vec<CharHit> = Vec::new();
    let mut total = 0;
    // Sort by count desc, then codepoint asc (mirror python ordering).
    let mut items: Vec<((u32, String), Vec<usize>)> = buckets.into_iter().collect();
    items.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0 .0.cmp(&b.0 .0)));
    for ((cp, kind), offsets) in items {
        total += offsets.len();
        hits.push(CharHit {
            codepoint: format!("U+{cp:04X}"),
            count: offsets.len(),
            confidence: hit_confidence(&kind).to_string(),
            kind,
            sample_offsets: offsets.into_iter().take(10).collect(),
        });
    }

    InspectReport {
        length: chars.len(),
        suspicious_total: total,
        hits,
    }
}
