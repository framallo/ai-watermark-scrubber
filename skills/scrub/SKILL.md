---
name: scrub
description: >-
  Detect and remove multi-vendor AI text watermarks and stray provenance
  metadata from text files and EPUBs using the `scrub` CLI (from the
  ai-watermark-scrubber project). Strips invisible-Unicode carriers (zero-width
  and format controls, bidi tricks), folds homoglyph spaces (NBSP, narrow NBSP,
  em/en) to plain spaces, and removes C2PA/`generator` metadata + repacks a valid
  EPUB. Trigger on "remove AI watermark", "check/clean invisible unicode",
  "strip zero-width characters", "de-watermark", "sanitize this text/EPUB before
  publishing", "does this file have hidden characters", "scrub this file", or
  cleaning a bookmill-built ebook. Preserves emoji ZWJ sequences and legitimate
  script joiners; does NOT touch statistical token-sampling watermarks or image
  pixel watermarks.
---

# scrub — remove AI text watermarks & hidden metadata

`scrub` is a single fast Rust binary that detects and removes the deterministic,
edit-based watermark carriers shared across many AI providers' text-watermark
schemes, plus provenance metadata in EPUB containers.

## When to use

- The user wants to know whether a file carries hidden/invisible characters or
  AI-watermark carriers.
- The user wants to clean a manuscript, article, or EPUB before publishing.
- Cleaning a bookmill-built ebook (`output/**/*.epub`).

## Prerequisite: is it installed?

```sh
command -v scrub    # if this prints a path, it's ready
```

If not installed, run the project's installer (`install.sh` in the
ai-watermark-scrubber repo) or `cargo install --path <repo>`. See "Install".

## Detect (never modifies the file)

```sh
scrub detect path/to/file.md          # exit code 1 if carriers/metadata found, 0 if clean
scrub detect path/to/book.epub        # scans every text entry + generator meta
scrub detect path/to/file.md --json   # machine-readable report
```

Use `detect` first to report findings. Exit code is the gate: non-zero = dirty.

## Clean

```sh
scrub clean path/to/file.md                 # writes file.cleaned.md (never overwrites by default)
scrub clean path/to/file.md -o out.md        # explicit output
scrub clean path/to/file.md --in-place       # overwrite the input
scrub clean path/to/book.epub --in-place     # clean text entries + strip generator meta, repack valid EPUB
```

Useful flags: `--nfkc` (Unicode NFKC normalize), `--aggressive-homoglyphs` (map
Cyrillic/fullwidth Latin lookalikes to ASCII), `--no-normalize-spaces`,
`--strip-emoji-glue`, `--strip-bidi`. Add `--json` for structured stats.

### Clean a whole directory of EPUBs

```sh
find output -name '*.epub' -print0 | xargs -0 -n1 scrub clean --in-place
```

## What it does / does not touch

- **Removes:** zero-width & format controls (ZWSP/ZWNJ/ZWJ free-floating, word
  joiner, BOM), bidi overrides, tag characters, variation selectors, private-use;
  folds NBSP/narrow-NBSP/em/en spaces to `U+0020`; strips `<meta
  name="generator">` and repacks EPUBs validly.
- **Preserves:** emoji ZWJ sequences (👨‍👩‍👧, ❤️‍🔥), CJK/Mongolian variation
  selectors, script joiners in Arabic/Indic/Khmer, flag tag sequences, paired
  bidi embeddings. Cleaning is idempotent.
- **Out of scope:** statistical token-sampling watermarks, image/audio pixel
  watermarks, PDF metadata.

## Use from Rust (library)

The crate is `ai-watermark-scrubber`; the library is imported as `scrub`:

```rust
use std::path::Path;
use scrub::{detect_file, clean_file_in_place, CleanOptions};

if !detect_file(Path::new("book.epub"))?.is_clean() {
    clean_file_in_place(Path::new("book.epub"), CleanOptions::default())?;
}
```

## Install

From the repo root:

```sh
./install.sh            # builds + installs the `scrub` binary and this skill
cargo install --path .  # binary only
```
