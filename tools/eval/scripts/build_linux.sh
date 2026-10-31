#!/usr/bin/env bash
set -euo pipefail

# Build a Linux glibc binary for the eval harness inside a container.
# The SWE-bench instance image verified here is Ubuntu 22.04 / glibc 2.35.
# The host may be arm64; build under linux/amd64 to match SWE-bench images.

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
out_dir="${repo_root}/target/eval-bin"
mkdir -p "${out_dir}"

# Prefer ubuntu:22.04; fall back to an already-pulled SWE-bench image when
# Docker Hub is unreachable.
base_image="${XYLITOL_EVAL_BUILD_IMAGE:-}"
if [[ -z "${base_image}" ]]; then
  base_image="ubuntu:22.04"
  if ! docker image inspect "${base_image}" --format '{{.Os}}/{{.Architecture}}' 2>/dev/null | grep -qx 'linux/amd64'; then
    if ! docker pull --platform linux/amd64 "${base_image}" >/dev/null 2>&1; then
      base_image="swebench/sweb.eval.x86_64.astropy_1776_astropy-12907:latest"
    fi
  fi
fi

docker run --rm \
  --platform linux/amd64 \
  -v "${repo_root}:/src" \
  -v "${out_dir}:/out" \
  -v "${HOME}/.cargo/registry:/root/.cargo/registry" \
  -v "${HOME}/.cargo/git:/root/.cargo/git" \
  -w /src \
  -e CARGO_TARGET_DIR=/tmp/xylitol-target \
  "${base_image}" \
  bash -lc '
    set -euo pipefail
    export DEBIAN_FRONTEND=noninteractive
    apt-get update
    apt-get install -y --no-install-recommends \
      build-essential clang mold cmake libssl-dev pkg-config perl nasm curl ca-certificates
    curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain nightly-2026-10-09
    . "$HOME/.cargo/env"
    rustup target add x86_64-unknown-linux-gnu
    cargo build --release --locked --target x86_64-unknown-linux-gnu
    cp "${CARGO_TARGET_DIR}/x86_64-unknown-linux-gnu/release/xylitol" /out/xylitol-linux-x86_64-glibc
    strip /out/xylitol-linux-x86_64-glibc
    sha256sum /out/xylitol-linux-x86_64-glibc
  '
