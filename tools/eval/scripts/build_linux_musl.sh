#!/usr/bin/env bash
set -euo pipefail

# Build a static musl Linux binary for Harbor tasks that are not Ubuntu/glibc.
# Uses an already-pulled SWE-bench image as the amd64 build base when Docker Hub
# is unreachable.

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
out_dir="${repo_root}/target/eval-bin"
mkdir -p "${out_dir}"

base_image="${XYLITOL_EVAL_BUILD_IMAGE:-swebench/sweb.eval.x86_64.astropy_1776_astropy-12907:latest}"

# The host proxy usually listens on 127.0.0.1; inside Docker that is the
# container itself. Rewrite loopback to the Docker host gateway.
proxy_url() {
  local value="${1:-}"
  value="${value//127.0.0.1/host.docker.internal}"
  value="${value//localhost/host.docker.internal}"
  printf '%s' "${value}"
}
https_proxy="$(proxy_url "${HTTPS_PROXY:-}")"
http_proxy="$(proxy_url "${HTTP_PROXY:-}")"
all_proxy="$(proxy_url "${ALL_PROXY:-}")"

docker run --rm \
  --platform linux/amd64 \
  -v "${repo_root}:/src" \
  -v "${out_dir}:/out" \
  -v "${HOME}/.cargo/registry:/root/.cargo/registry" \
  -v "${HOME}/.cargo/git:/root/.cargo/git" \
  -w /src \
  -e CARGO_TARGET_DIR=/tmp/xylitol-musl-target \
  --add-host=host.docker.internal:host-gateway \
  -e HTTPS_PROXY="${https_proxy}" \
  -e HTTP_PROXY="${http_proxy}" \
  -e ALL_PROXY="${all_proxy}" \
  -e NO_PROXY="${NO_PROXY:-}" \
  "${base_image}" \
  bash -lc '
    set -euo pipefail
    export DEBIAN_FRONTEND=noninteractive
    apt-get update
    apt-get install -y --no-install-recommends \
      build-essential clang mold cmake libssl-dev pkg-config perl nasm musl-tools curl ca-certificates file
    curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain nightly-2026-10-09
    . "$HOME/.cargo/env"
    rustup target add x86_64-unknown-linux-musl
    cargo build --release --locked --target x86_64-unknown-linux-musl
    cp "${CARGO_TARGET_DIR}/x86_64-unknown-linux-musl/release/xylitol" /out/xylitol-linux-x86_64-musl
    strip /out/xylitol-linux-x86_64-musl
    file /out/xylitol-linux-x86_64-musl
    ldd /out/xylitol-linux-x86_64-musl || true
    sha256sum /out/xylitol-linux-x86_64-musl
  '
