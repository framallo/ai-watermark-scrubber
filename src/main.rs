//! scrub CLI: `scrub detect <path>` and `scrub clean <path>`.
//!
//! Handles UTF-8 text files and EPUB containers (detected by `.epub`). `detect`
//! exits non-zero when it finds carriers/metadata; `clean` exits 0 on success.

use anyhow::{bail, Context, Result};
use clap::{Args, Parser, Subcommand};
use scrub::epub::{clean_epub, inspect_epub};
use scrub::unicode::{clean_text, inspect_text, CleanOptions};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "scrub", version, about = "Detect and clean invisible-Unicode carriers and stray metadata")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Report carriers/metadata without modifying the file. Exits 1 on findings.
    Detect {
        path: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Clean the file in place or to an output path.
    Clean(CleanArgs),
}

#[derive(Args)]
struct CleanArgs {
    path: PathBuf,
    /// Output path (default: overwrite in place is off; writes <name>.cleaned.<ext>).
    #[arg(short, long)]
    output: Option<PathBuf>,
    /// Overwrite the input file.
    #[arg(long)]
    in_place: bool,
    #[arg(long)]
    nfkc: bool,
    #[arg(long)]
    aggressive_homoglyphs: bool,
    #[arg(long)]
    no_normalize_spaces: bool,
    #[arg(long)]
    strip_emoji_glue: bool,
    #[arg(long)]
    strip_bidi: bool,
    #[arg(long)]
    json: bool,
}

fn is_epub(path: &Path) -> bool {
    path.extension()
        .map(|e| e.eq_ignore_ascii_case("epub"))
        .unwrap_or(false)
}

fn cleaned_path(path: &Path) -> PathBuf {
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("out");
    let ext = path.extension().and_then(|s| s.to_str());
    let name = match ext {
        Some(e) => format!("{stem}.cleaned.{e}"),
        None => format!("{stem}.cleaned"),
    };
    path.with_file_name(name)
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Detect { path, json } => detect(&path, json),
        Cmd::Clean(a) => clean(a),
    }
}

fn detect(path: &Path, json: bool) -> Result<()> {
    if !path.is_file() {
        bail!("not a file: {}", path.display());
    }
    let findings = if is_epub(path) {
        let rep = inspect_epub(path)?;
        let found = rep.suspicious_total > 0 || rep.generator_meta_total > 0;
        if json {
            println!("{}", serde_json::to_string_pretty(&rep)?);
        } else if found {
            eprintln!(
                "{}: {} carrier(s) across {} entr(ies), {} generator-meta tag(s)",
                path.display(),
                rep.suspicious_total,
                rep.findings.len(),
                rep.generator_meta_total
            );
            for f in &rep.findings {
                eprintln!(
                    "  {}: suspicious={} generator_meta={}",
                    f.entry, f.suspicious_total, f.generator_meta
                );
            }
        } else {
            eprintln!("{}: clean", path.display());
        }
        found
    } else {
        let text = read_utf8(path)?;
        let rep = inspect_text(&text, false, false);
        let found = rep.suspicious_total > 0;
        if json {
            println!("{}", serde_json::to_string_pretty(&rep)?);
        } else if found {
            eprintln!("{}: {} suspicious codepoint(s)", path.display(), rep.suspicious_total);
            for h in &rep.hits {
                eprintln!("  {} {} x{} [{}]", h.codepoint, h.kind, h.count, h.confidence);
            }
        } else {
            eprintln!("{}: clean", path.display());
        }
        found
    };
    if findings {
        std::process::exit(1);
    }
    Ok(())
}

fn clean(a: CleanArgs) -> Result<()> {
    if !a.path.is_file() {
        bail!("not a file: {}", a.path.display());
    }
    let opts = CleanOptions {
        nfkc: a.nfkc,
        aggressive_homoglyphs: a.aggressive_homoglyphs,
        normalize_spaces: !a.no_normalize_spaces,
        strip_emoji_glue: a.strip_emoji_glue,
        strip_bidi: a.strip_bidi,
    };

    let dest = if a.in_place {
        a.path.clone()
    } else {
        a.output.clone().unwrap_or_else(|| cleaned_path(&a.path))
    };

    if is_epub(&a.path) {
        // Clean to a temp file, then move into place (safe even when dest == src).
        let tmp = dest.with_extension("epub.scrub.tmp");
        let stats = clean_epub(&a.path, &tmp, opts)?;
        std::fs::rename(&tmp, &dest)
            .with_context(|| format!("moving cleaned EPUB into {}", dest.display()))?;
        if a.json {
            println!("{}", serde_json::to_string_pretty(&stats)?);
        } else {
            eprintln!(
                "{} -> {}: removed={} replaced={} generator_meta_removed={} ({} text entries)",
                a.path.display(),
                dest.display(),
                stats.removed_chars,
                stats.replaced_chars,
                stats.generator_meta_removed,
                stats.entries_text
            );
        }
    } else {
        let text = read_utf8(&a.path)?;
        let (cleaned, stats) = clean_text(&text, opts);
        std::fs::write(&dest, cleaned.as_bytes())
            .with_context(|| format!("writing {}", dest.display()))?;
        if a.json {
            println!("{}", serde_json::to_string_pretty(&stats)?);
        } else {
            eprintln!(
                "{} -> {}: removed={} replaced={} len {}->{}",
                a.path.display(),
                dest.display(),
                stats.removed_count,
                stats.replaced_count,
                stats.input_length,
                stats.output_length
            );
        }
    }
    Ok(())
}

fn read_utf8(path: &Path) -> Result<String> {
    let raw = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    String::from_utf8(raw).with_context(|| format!("{} is not valid UTF-8 text", path.display()))
}
