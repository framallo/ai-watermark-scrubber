//! scrub CLI: `scrub detect <path>` and `scrub clean <path>`.
//!
//! A thin front end over the `scrub` library. Handles UTF-8 text files and EPUB
//! containers (detected by `.epub`). `detect` exits non-zero when it finds
//! carriers/metadata; `clean` exits 0 on success.

use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use scrub::api::{clean_file, cleaned_path, detect_file, CleanReport, DetectReport};
use scrub::CleanOptions;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "scrub", version, about = "Detect and clean invisible-Unicode carriers and stray (incl. AI-watermark) metadata")]
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
    /// Output path (default: writes <name>.cleaned.<ext> next to the input).
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

fn main() -> Result<()> {
    match Cli::parse().cmd {
        Cmd::Detect { path, json } => detect(path, json),
        Cmd::Clean(a) => clean(a),
    }
}

fn detect(path: PathBuf, json: bool) -> Result<()> {
    let report = detect_file(&path)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else if report.is_clean() {
        eprintln!("{}: clean", path.display());
    } else {
        match &report {
            DetectReport::Text(r) => {
                eprintln!("{}: {} suspicious codepoint(s)", path.display(), r.suspicious_total);
                for h in &r.hits {
                    eprintln!("  {} {} x{} [{}]", h.codepoint, h.kind, h.count, h.confidence);
                }
            }
            DetectReport::Epub(r) => {
                eprintln!(
                    "{}: {} carrier(s) across {} entr(ies), {} generator-meta tag(s)",
                    path.display(),
                    r.suspicious_total,
                    r.findings.len(),
                    r.generator_meta_total
                );
                for f in &r.findings {
                    eprintln!(
                        "  {}: suspicious={} generator_meta={}",
                        f.entry, f.suspicious_total, f.generator_meta
                    );
                }
            }
        }
    }
    if !report.is_clean() {
        std::process::exit(1);
    }
    Ok(())
}

fn clean(a: CleanArgs) -> Result<()> {
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

    let report = clean_file(&a.path, &dest, opts)?;
    if a.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        match &report {
            CleanReport::Text(s) => eprintln!(
                "{} -> {}: removed={} replaced={} len {}->{}",
                a.path.display(),
                dest.display(),
                s.removed_count,
                s.replaced_count,
                s.input_length,
                s.output_length
            ),
            CleanReport::Epub(s) => eprintln!(
                "{} -> {}: removed={} replaced={} generator_meta_removed={} ({} text entries)",
                a.path.display(),
                dest.display(),
                s.removed_chars,
                s.replaced_chars,
                s.generator_meta_removed,
                s.entries_text
            ),
        }
    }
    Ok(())
}
