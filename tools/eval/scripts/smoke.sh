#!/usr/bin/env bash
set -euo pipefail

mode="run"
case "${1:-}" in
  "") ;;
  --check|-c|check) mode="check" ;;
  *) echo "usage: $0 [--check]" >&2; exit 2 ;;
esac

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || true)"
if [[ -z "${repo_root}" ]]; then
  echo "error: not inside a git repository" >&2
  exit 2
fi
cd "${repo_root}"

harbor_bin="${HARBOR_BIN:-/tmp/harbor-venv/bin/harbor}"
binary="target/eval-bin/xylitol-linux-x86_64-musl"
fixture="tools/eval/fixtures/smoke-task"
env_file="${MAGPIE_HARBOR_ENV_FILE:-/tmp/magpie-harbor.env}"

fail() {
  echo "smoke check failed: $*" >&2
  exit 1
}

[[ -x "${harbor_bin}" ]] || fail "Harbor not executable at ${harbor_bin}"
[[ -x "${binary}" ]] || fail "xylitol binary missing at ${binary}"
[[ -f "${fixture}/task.toml" ]] || fail "fixture task.toml missing"
[[ -x "tools/eval/scripts/harbor_local.sh" ]] || fail "harbor_local.sh missing"
[[ -f "${env_file}" ]] || fail "env file missing at ${env_file}"

harbor_version="$("${harbor_bin}" --version 2>/dev/null)" || fail "Harbor --version failed"
binary_sha="$(python3 - "${binary}" <<'PY'
import hashlib, sys
h = hashlib.sha256()
with open(sys.argv[1], "rb") as f:
    for chunk in iter(lambda: f.read(1024 * 1024), b""):
        h.update(chunk)
print(h.hexdigest())
PY
)" || fail "binary SHA failed"

grep -q '^MAGPIE_GATEWAY_URL=' "${env_file}" || fail "MAGPIE_GATEWAY_URL missing from ${env_file}"
grep -q '^MAGPIE_GATEWAY_KEY=' "${env_file}" || fail "MAGPIE_GATEWAY_KEY missing from ${env_file}"
docker info >/dev/null 2>&1 || fail "Docker is not available"

image="$(sed -n 's/^docker_image = "\(.*\)"/\1/p' "${fixture}/task.toml" | head -n 1)"
[[ -n "${image}" ]] || fail "docker_image missing from ${fixture}/task.toml"
if [[ "${mode}" == "run" ]]; then
  platform="$(docker image inspect "${image}" --format '{{.Os}}/{{.Architecture}}' 2>/dev/null || true)"
  if [[ "${platform}" != "linux/amd64" ]]; then
    docker pull --platform linux/amd64 "${image}" >/dev/null || fail "cannot pull amd64 image ${image}"
  fi
fi
platform="$(docker image inspect "${image}" --format '{{.Os}}/{{.Architecture}}' 2>/dev/null || true)"
[[ "${platform}" == "linux/amd64" ]] || fail "image ${image} is ${platform:-missing}, need linux/amd64"

"${harbor_bin}" lint --ignore unpinned-image --ignore shared-verifier "${fixture}" >/dev/null   || fail "Harbor lint failed for ${fixture}"

echo "smoke check ok: harbor=${harbor_version} binary_sha=${binary_sha} fixture=${fixture}"
if [[ "${mode}" == "check" ]]; then
  exit 0
fi

set -a
. "${env_file}"
set +a

job_name="smoke-$(date +%Y%m%d-%H%M%S)"
export DOCKER_BUILDKIT=0 COMPOSE_DOCKER_CLI_BUILD=0 DOCKER_DEFAULT_PLATFORM=linux/amd64

if ! HARBOR_BIN="${harbor_bin}" tools/eval/scripts/harbor_local.sh run \
  -p "${fixture}" \
  -a xylitol_agent:XylitolAgent \
  -m openai/group/glm-5.3-flash \
  --ak binary="${repo_root}/${binary}" \
  --ak max_turns=8 \
  --ak wall_seconds=120 \
  --base-url "${MAGPIE_GATEWAY_URL}/v1" \
  --api-key-name MAGPIE_GATEWAY_KEY \
  --api-format openai_responses \
  --env-file "${env_file}" \
  --job-name "${job_name}" \
  -k 1 -n 1 -y; then
  echo "smoke run failed: jobs/${job_name}" >&2
  exit 1
fi

python3 - "${repo_root}/jobs/${job_name}" <<'PY'
import glob, json, os, sys

job_dir = sys.argv[1]
with open(os.path.join(job_dir, "result.json")) as f:
    job = json.load(f)
stats = job.get("stats", {})
if stats.get("n_completed_trials") != 1 or stats.get("n_errored_trials") != 0:
    raise SystemExit(f"unexpected job stats: {stats}")

trials = glob.glob(os.path.join(job_dir, "*", "result.json"))
if len(trials) != 1:
    raise SystemExit(f"expected 1 trial result, got {len(trials)}")
with open(trials[0]) as f:
    trial = json.load(f)
if trial.get("exception_info") is not None:
    raise SystemExit(f"trial exception: {trial['exception_info']}")
reward = (trial.get("verifier_result") or {}).get("rewards", {}).get("reward")
if reward != 1:
    logs = glob.glob(os.path.join(job_dir, "*", "agent", "xylitol.txt"))
    text = ""
    for path in logs:
        with open(path, errors="replace") as f:
            text += f.read()
    if "429" in text or "rate_limit" in text:
        raise SystemExit("smoke blocked: model quota/rate limit; see agent log")
    raise SystemExit(f"expected reward 1, got {reward!r}")
print(f"smoke passed: {job_dir} reward={reward}")
PY
