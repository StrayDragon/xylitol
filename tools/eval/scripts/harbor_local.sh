#!/usr/bin/env bash
set -euo pipefail

# Local-only Harbor wrapper. Do not wire this into CI.
#
# - translates a loopback HTTP(S) proxy to host.docker.internal so Pi/nvm and
#   mini-SWE/uv installs can reach the network;
# - adds NO_PROXY for the magpie gateway so model calls stay direct;
# - refuses proxy URLs with credentials because `--ae` lands in argv;
# - caps local Harbor concurrency at 3 (default 3).

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

# Local concurrency cap: at most 3 Harbor trials at once.
normalized=()
concurrency=""
i=0
while (( i < ${#args[@]} )); do
  arg="${args[i]}"
  case "${arg}" in
    -n|--n-concurrent)
      if (( i + 1 >= ${#args[@]} )); then
        echo "error: ${arg} requires a value" >&2
        exit 2
      fi
      value="${args[i+1]}"
      if ! [[ "${value}" =~ ^[0-9]+$ ]]; then
        echo "error: ${arg} must be an integer" >&2
        exit 2
      fi
      if (( value < 1 || value > 3 )); then
        echo "error: local concurrency is capped at 3 (got ${value})" >&2
        exit 2
      fi
      concurrency="${value}"
      normalized+=("${arg}" "${value}")
      (( i += 2 ))
      ;;
    -n[0-9]*|-n=*)
      value="${arg#-n}"
      value="${value#=}"
      if ! [[ "${value}" =~ ^[0-9]+$ ]]; then
        echo "error: -n must be an integer" >&2
        exit 2
      fi
      if (( value < 1 || value > 3 )); then
        echo "error: local concurrency is capped at 3 (got ${value})" >&2
        exit 2
      fi
      concurrency="${value}"
      normalized+=("-n" "${value}")
      (( i += 1 ))
      ;;
    --n-concurrent=*)
      value="${arg#*=}"
      if ! [[ "${value}" =~ ^[0-9]+$ ]]; then
        echo "error: --n-concurrent must be an integer" >&2
        exit 2
      fi
      if (( value < 1 || value > 3 )); then
        echo "error: local concurrency is capped at 3 (got ${value})" >&2
        exit 2
      fi
      concurrency="${value}"
      normalized+=("${arg}")
      (( i += 1 ))
      ;;
    *)
      normalized+=("${arg}")
      (( i += 1 ))
      ;;
  esac
done
if [[ -z "${concurrency}" ]]; then
  normalized+=("-n" "3")
fi
args=("${normalized[@]}")

if [[ -n "${proxy}" ]]; then
  args+=(
    --ae "HTTPS_PROXY=${proxy}"
    --ae "HTTP_PROXY=${proxy}"
    --ae "ALL_PROXY=${proxy}"
    --ae "NO_PROXY=${no_proxy}"
  )
fi
if [[ -n "${gateway_host}" ]]; then
  args+=(--allow-agent-host "${gateway_host}")
fi
args+=(--allow-agent-host host.docker.internal)

export PYTHONPATH="${repo_root}/tools/eval/harbor${PYTHONPATH:+:${PYTHONPATH}}"
exec "${harbor_bin}" "${args[@]}"
