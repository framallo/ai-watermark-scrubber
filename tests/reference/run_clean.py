#!/usr/bin/env python3
"""Reference harness for differential testing.

Reads UTF-8 from stdin, runs the vendored Layer A `clean_text`, writes the
cleaned UTF-8 to stdout with NO added trailing newline (so the bytes match the
Rust port exactly). Flags mirror the Rust `CleanOptions`.
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from text_unicode import clean_text  # noqa: E402


def main() -> int:
    flags = set(sys.argv[1:])
    text = sys.stdin.buffer.read().decode("utf-8", errors="surrogateescape")
    cleaned, _stats = clean_text(
        text,
        nfkc="--nfkc" in flags,
        aggressive_homoglyphs="--aggressive-homoglyphs" in flags,
        normalize_spaces="--no-normalize-spaces" not in flags,
        strip_emoji_glue="--strip-emoji-glue" in flags,
        strip_bidi="--strip-bidi" in flags,
    )
    sys.stdout.buffer.write(cleaned.encode("utf-8", errors="surrogateescape"))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
