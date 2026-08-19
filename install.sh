#!/usr/bin/env bash
# Install the `scrub` binary and (optionally) the agent skill on this machine.
#
# Usage:
#   ./install.sh                 # install binary + skill for every detected agent
#   ./install.sh --no-skill      # binary only
#   ./install.sh --skill-only    # skill only (skip building the binary)
#   ./install.sh --skill-dir DIR # also install the skill into DIR
#
# Works on macOS and Linux. The binary is installed with `cargo install` into
# ~/.cargo/bin (ensure that is on your PATH).

set -euo pipefail

REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SKILL_SRC="$REPO_DIR/skills/scrub"

DO_BINARY=1
DO_SKILL=1
EXTRA_SKILL_DIRS=()

while [ $# -gt 0 ]; do
  case "$1" in
    --no-skill)    DO_SKILL=0 ;;
    --skill-only)  DO_BINARY=0 ;;
    --skill-dir)   shift; EXTRA_SKILL_DIRS+=("$1") ;;
    -h|--help)     grep '^#' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "unknown option: $1" >&2; exit 2 ;;
  esac
  shift
done

info()  { printf '\033[1;34m==>\033[0m %s\n' "$*"; }
ok()    { printf '\033[1;32m  ok\033[0m %s\n' "$*"; }
warn()  { printf '\033[1;33m  !!\033[0m %s\n' "$*" >&2; }

install_binary() {
  if ! command -v cargo >/dev/null 2>&1; then
    warn "cargo not found. Install the Rust toolchain first:"
    warn "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    warn "then re-run: $0"
    exit 1
  fi
  info "Building and installing the 'scrub' binary (cargo install)…"
  cargo install --path "$REPO_DIR" --force
  if command -v scrub >/dev/null 2>&1; then
    ok "$(command -v scrub) — $(scrub --version)"
  else
    warn "'scrub' installed to ~/.cargo/bin but not on PATH."
    warn "Add this to your shell profile:  export PATH=\"\$HOME/.cargo/bin:\$PATH\""
  fi
}

link_skill() {
  local dest_parent="$1"
  local dest="$dest_parent/scrub"
  mkdir -p "$dest_parent"
  # Symlink so future `git pull`s update the skill in place.
  ln -sfn "$SKILL_SRC" "$dest"
  ok "skill -> $dest"
}

install_skills() {
  local installed=0
  # Known agent skill locations. A dir is targeted when its parent exists,
  # so we only touch agents that are actually set up on this machine.
  local candidates=(
    "$HOME/.claude/skills"     # Claude Code
    "$HOME/.grok/skills"       # Grok
    "$HOME/.codex/skills"      # Codex-style agents
    "$HOME/.config/agent/skills"
  )
  for parent in "${candidates[@]}"; do
    local base; base="$(dirname "$parent")"   # e.g. ~/.claude
    if [ -d "$base" ]; then
      link_skill "$parent"; installed=1
    fi
  done
  for dir in "${EXTRA_SKILL_DIRS[@]:-}"; do
    [ -n "$dir" ] && { link_skill "$dir"; installed=1; }
  done
  if [ "$installed" -eq 0 ]; then
    warn "no agent skill directories detected (Claude Code, Grok, …)."
    warn "install manually with:  ./install.sh --skill-dir <path-to>/skills"
  fi
}

info "ai-watermark-scrubber installer ($REPO_DIR)"
[ "$DO_BINARY" -eq 1 ] && install_binary || info "skipping binary (--skill-only)"
[ "$DO_SKILL"  -eq 1 ] && install_skills || info "skipping skill (--no-skill)"
info "Done."
