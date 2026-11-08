#!/usr/bin/env bash
set -euo pipefail

# Build a static musl Linux binary for Harbor tasks.
# The build base is an amd64 Ubuntu image; override with
# XYLITOL_EVAL_BUILD_IMAGE when Docker Hub is unreachable.

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
out_dir="${repo_root}/target/eval-bin"
mkdir -p "${out_dir}"

# target may be a symlink (mbx cache). Mount its real path so the codemode
# guest build can create target/codemode-guest inside the container.
target_real="$(realpath "${repo_root}/target")"

base_image="${XYLITOL_EVAL_BUILD_IMAGE:-ubuntu:22.04}"

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

if [[ "${https_proxy}" == *"@"* || "${http_proxy}" == *"@"* || "${all_proxy}" == *"@"* ]]; then
  echo "error: proxy URL contains credentials; refusing to pass it via docker argv" >&2
  exit 2
fi

docker run --rm \
  --platform linux/amd64 \
  -v "${repo_root}:/src" \
  -v "${target_real}:/src/target" \
  -v "${out_dir}:/out" \
  -v "${HOME}/.cargo/registry:/root/.cargo/registry" \
  -v "${HOME}/.cargo/git:/root/.cargo/git" \
  -w /src \
  -e CARGO_TARGET_DIR=/src/target/xylitol-musl-target \
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
    curl -fsSL -o /tmp/wasi-sdk.tar.gz https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-24/wasi-sdk-24.0-x86_64-linux.tar.gz
    mkdir -p /opt/wasi-sdk
    tar -xzf /tmp/wasi-sdk.tar.gz -C /opt/wasi-sdk --strip-components=1
    export WASI_SDK_PATH=/opt/wasi-sdk
    rustup target add x86_64-unknown-linux-musl wasm32-wasip1
    cargo build --release --locked --target x86_64-unknown-linux-musl
    cp "${CARGO_TARGET_DIR}/x86_64-unknown-linux-musl/release/xylitol" /out/xylitol-linux-x86_64-musl
    strip /out/xylitol-linux-x86_64-musl
    file /out/xylitol-linux-x86_64-musl
    ldd /out/xylitol-linux-x86_64-musl || true
    sha256sum /out/xylitol-linux-x86_64-musl
  '
