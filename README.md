# ai-watermark-scrubber

A small, fast Rust tool that **detects and removes multi-vendor AI text
watermarks** — invisible-Unicode carriers and stray provenance metadata — from
text files and EPUB containers. It is a Rust port of the deterministic Layer A
logic from
[`watermarks-remover`](https://github.com/guillaumemeyer/watermarks-remover),
scoped to the testable cases and to the EPUB output of a book pipeline.

The binary and the library both keep the short name **`scrub`** — you run
`scrub …` and write `use scrub::…` — while the crate advertises what it does.

> **Multi-brand:** the invisible-Unicode carriers stripped here are the shared
> substrate that many AI providers' text-watermark schemes ride on (zero-width
> and format controls, homoglyph spacing, bidi tricks), plus C2PA/generator
> provenance tags in EPUB metadata. Statistical token-sampling watermarks are
> out of scope (see below).

## What it does

- **Text (Layer A):** strips zero-width and format controls (ZWSP, ZWNJ/ZWJ,
  word joiner, BOM, bidi overrides, tag characters, variation selectors, private
  use), folds exotic space homoglyphs (NBSP, narrow NBSP, em/en spaces, …) to a
  plain `U+0020`, and — with `--aggressive-homoglyphs` — maps Cyrillic/fullwidth
  Latin lookalikes to ASCII.
- **Preserves load-bearing invisibles:** emoji ZWJ sequences (👨‍👩‍👧, ❤️‍🔥),
  CJK/Mongolian variation selectors, script joiners in Arabic/Indic/Khmer,
  complete flag tag sequences, paired bidi embeddings, and orthographic Arabic
  Cf marks. Cleaning is idempotent.
- **EPUB:** cleans every text entry (xhtml/opf/ncx/css/svg), removes stray
  `<meta name="generator">` tags, and repacks a valid OCF container (`mimetype`
  first and STORED, binary entries byte-identical). Verified against
  `epubcheck`.

Out of scope (by design): statistical/token-sampling watermark rewriting,
image/audio pixel watermarks, and PDF metadata surgery.

## Usage

```sh
scrub detect path/to/file.md        # exit 1 if carriers/metadata found, else 0
scrub detect path/to/book.epub --json
scrub clean  path/to/file.md -o out.md
scrub clean  path/to/book.epub --in-place
```

`detect` never writes; `clean` writes to `-o`, `--in-place`, or a
`*.cleaned.*` sibling by default.

## Use as a Rust library

Add the crate (the library is imported as `scrub`):

```toml
[dependencies]
ai-watermark-scrubber = "0.1"
```

High-level, file-oriented API:

```rust
use std::path::Path;
use scrub::{detect_file, clean_file_in_place, CleanOptions};

let report = detect_file(Path::new("book.epub"))?;
if !report.is_clean() {
    let stats = clean_file_in_place(Path::new("book.epub"), CleanOptions::default())?;
    println!("removed {} carrier(s)", stats.removed());
}
# Ok::<(), anyhow::Error>(())
```

In-memory, no filesystem:

```rust
use scrub::{clean_text, inspect_text, CleanOptions};

let (clean, stats) = clean_text("in\u{200B}visible\u{00A0}text", CleanOptions::default());
assert_eq!(clean, "invisible text");

let report = inspect_text("plain text", false, false);
assert!(report.suspicious_total == 0);
```

Lower-level modules: `scrub::unicode` (text) and `scrub::epub` (containers).

## Tests

```sh
cargo test
```

- `unicode_tests` — behavioral unit tests for the cleaner/detector.
- `differential` — asserts the Rust output is **byte-identical** to the vendored
  Python reference across a carrier/preservation corpus × every option
  combination (skipped if `python3` is absent).
- `epub_tests` — injected carriers/metadata are removed, binary entries pass
  through unchanged, the repack is OCF-valid, and (when installed) `epubcheck`
  accepts the cleaned EPUB.
- `e2e_hook` — runs the real `scrub` binary exactly as a build pipeline hook
  would (`scrub clean --in-place <file>`) and proves a watermarked EPUB comes
  out clean and valid.
- `library_api` — drives the high-level `detect_file` / `clean_file` API on both
  a text file and an EPUB, plus doctests on the crate's public API.

## Install

One command builds the `scrub` binary and registers the agent skill wherever a
supported agent is set up on the machine (Claude Code, Grok, …):

```sh
./install.sh                 # binary + skill (auto-detects agent skill dirs)
./install.sh --no-skill      # binary only
./install.sh --skill-only    # skill only
./install.sh --skill-dir <path>/skills   # also install the skill somewhere custom
```

The binary lands in `~/.cargo/bin/scrub` (make sure that's on your `PATH`). The
skill is **symlinked** from the repo, so a later `git pull` updates it in place.
Binary-only, without the script:

```sh
cargo install --path .       # from a clone
cargo install --git https://github.com/framallo/ai-watermark-scrubber   # from git
```

### Agent skill

The repo ships a `scrub` skill (`skills/scrub/SKILL.md`). Once installed, an
agent like Claude Code invokes `scrub` automatically for requests like "remove
AI watermarks from this file", "does this have hidden characters?", or "clean
this EPUB before publishing". The installer links it into `~/.claude/skills/`
(and any other detected agent dirs).

## Use as a build hook

`scrub` is invoked by [bookmill](../libs/bookmill) via a generic post-build
hook. In a repo's `bookmill.toml`:

```toml
[[hooks.post_build]]
command = "scrub"
args = ["clean", "--in-place", "{file}"]
formats = ["epub"]
required = true
```

Install it so the pipeline can find it on `PATH` (`./install.sh` or `cargo
install --path .`).

`scrub` is invoked by [bookmill](../libs/bookmill) via a generic post-build
hook. In a repo's `bookmill.toml`:

```toml
[[hooks.post_build]]
command = "scrub"
args = ["clean", "--in-place", "{file}"]
formats = ["epub"]
required = true
```

Install it so the pipeline can find it on `PATH`:

```sh
cargo install --path .
```
