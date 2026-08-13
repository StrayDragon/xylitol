#!/usr/bin/env bash
# Isolate CARGO_TARGET_DIR per git worktree (or checkout path).
#
# Why: sharing one target dir across worktrees of the SAME crate can make Cargo
# report Fresh while running another tree's artifacts (verified on Cargo 1.97.1).
# Share registry/git/sccache globally; keep target private per worktree.
#
# Usage:
#   source scripts/cargo_worktree_env.sh
#   source scripts/cargo_worktree_env.sh --print   # only print exports
#   eval "$(scripts/cargo_worktree_env.sh --print)"
#
# Optional:
#   CARGO_TARGET_CACHE_ROOT=~/.cache/cargo-targets  (default)
#   RUSTC_WRAPPER=sccache   # set yourself if installed; safe across worktrees
#   SCCACHE_CACHE_SIZE=60G  # defaulted below when unset — caps sccache disk growth
#
# See: docs/research/rust-disk-worktree-cache-2026.md

set -euo pipefail

PRINT_ONLY=0
for arg in "$@"; do
  case "$arg" in
    --print|-p) PRINT_ONLY=1 ;;
    -h|--help)
      sed -n '1,22p' "$0"
      exit 0
      ;;
    *)
      echo "error: unknown arg: $arg (use --print)" >&2
      exit 2
      ;;
  esac
done

# Resolve repo root: prefer git, fall back to script location.
if ROOT="$(git rev-parse --show-toplevel 2>/dev/null)"; then
  :
else
  ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fi

REPO_NAME="$(basename "$ROOT")"
# Prefer worktree path identity over branch name (branches can collide).
WT_KEY="$(printf '%s' "$ROOT" | sha1sum | awk '{print substr($1,1,12)}')"

CACHE_ROOT="${CARGO_TARGET_CACHE_ROOT:-${HOME}/.cache/cargo-targets}"
TARGET_DIR="${CACHE_ROOT}/${REPO_NAME}/${WT_KEY}"
mkdir -p "$TARGET_DIR"

# Cap sccache so the shared compile cache cannot grow without bound.
# Override with SCCACHE_CACHE_SIZE=… before sourcing if needed.
if [[ -z "${SCCACHE_CACHE_SIZE:-}" ]]; then
  SCCACHE_CACHE_SIZE=60G
fi

# CACHEDIR.TAG so backup tools can skip (Cargo also writes one under target/).
if [[ ! -f "${TARGET_DIR}/CACHEDIR.TAG" ]]; then
  printf '%s\n' \
    'Signature: 8a477f597d28d172789f06886806bc55' \
    '# This file is a cache directory tag created by xylitol cargo_worktree_env.' \
    '# For information about cache directory tags, see:' \
    '# 	https://bford.info/cachedir/' \
    >"${TARGET_DIR}/CACHEDIR.TAG"
fi

emit() {
  echo "export CARGO_TARGET_DIR=$(printf '%q' "$TARGET_DIR")"
  echo "export SCCACHE_CACHE_SIZE=$(printf '%q' "$SCCACHE_CACHE_SIZE")"
  echo "# repo=${REPO_NAME} worktree_key=${WT_KEY}"
  if command -v sccache >/dev/null 2>&1; then
    echo "export RUSTC_WRAPPER=sccache"
  else
    echo "# tip: install sccache and re-run for RUSTC_WRAPPER=sccache"
  fi
}

if [[ "$PRINT_ONLY" -eq 1 ]]; then
  emit
  exit 0
fi

# When sourced: apply to current shell.
if [[ "${BASH_SOURCE[0]}" != "${0}" ]]; then
  export CARGO_TARGET_DIR="$TARGET_DIR"
  export SCCACHE_CACHE_SIZE
  if command -v sccache >/dev/null 2>&1; then
    export RUSTC_WRAPPER=sccache
  fi
  echo "CARGO_TARGET_DIR=$CARGO_TARGET_DIR" >&2
  echo "SCCACHE_CACHE_SIZE=$SCCACHE_CACHE_SIZE" >&2
  return 0 2>/dev/null || true
fi

# When executed: print for eval.
emit
