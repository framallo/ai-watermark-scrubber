# scrub

A small, fast Rust tool that **detects and cleans invisible-Unicode carriers and
stray provenance metadata** from text files and EPUB containers. It is a Rust
port of the Layer A logic from
[`watermarks-remover`](https://github.com/guillaumemeyer/watermarks-remover),
scoped to the deterministic, testable cases and to the EPUB output of a book
pipeline.

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

Install it so the pipeline can find it on `PATH`:

```sh
cargo install --path .
```
