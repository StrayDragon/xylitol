#!/usr/bin/env bash
set -euo pipefail

# Local-only Harbor wrapper. Do not wire this into CI.
#
# - translates a loopback HTTP(S) proxy to host.docker.internal so Pi/nvm and
#   mini-SWE/uv installs can reach the network;
# - adds NO_PROXY for the magpie gateway so model calls stay direct;
# - refuses proxy URLs with credentials because `--ae` lands in argv.

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
harbor_bin="${HARBOR_BIN:-/tmp/harbor-venv/bin/harbor}"
if [[ ! -x "${harbor_bin}" ]]; then
  echo "error: Harbor not found at ${harbor_bin}" >&2
  exit 1
fi

if [[ "${1:-}" != "run" ]]; then
  exec "${harbor_bin}" "$@"
fi

proxy="${HTTPS_PROXY:-${HTTP_PROXY:-${ALL_PROXY:-}}}"
if [[ -n "${proxy}" ]]; then
  proxy="${proxy//127.0.0.1/host.docker.internal}"
  proxy="${proxy//localhost/host.docker.internal}"
  if [[ "${proxy}" == *"@"* ]]; then
    echo "error: proxy URL contains credentials; refusing to pass it via argv" >&2
    exit 2
  fi
fi

gateway_host=""
if [[ -n "${MAGPIE_GATEWAY_URL:-}" ]]; then
  gateway_host="$(python3 - "${MAGPIE_GATEWAY_URL}" <<'PY'
import sys
from urllib.parse import urlsplit

host = urlsplit(sys.argv[1]).hostname
if host:
    print(host)
PY
)"
fi

no_proxy="${NO_PROXY:-}"
for host in host.docker.internal 127.0.0.1 localhost "${gateway_host}"; do
  [[ -n "${host}" ]] || continue
  case ",${no_proxy}," in
    *",${host},"*) ;;
    *) no_proxy="${no_proxy:+${no_proxy},}${host}" ;;
  esac
done

args=("$@")
if [[ -n "${proxy}" ]]; then
  args+=(
    --ae "HTTPS_PROXY=${proxy}"
    --ae "HTTP_PROXY=${proxy}"
    --ae "ALL_PROXY=${proxy}"
    --ae "NO_PROXY=${no_proxy}"
  )
fi
args+=(--allow-agent-host host.docker.internal)

export PYTHONPATH="${repo_root}/tools/eval/harbor${PYTHONPATH:+:${PYTHONPATH}}"
exec "${harbor_bin}" "${args[@]}"
