# xylitol-eval

Headless agent eval harness for xylitol. Developer tooling, not a product
feature.

## Contract

- Scores come only from the official Harbor task verifier. No self-made judge.
- Drive the agent only through `xylitol print` (headless). Do not drive the TUI
  or send keystrokes.
- Pin the model, dataset revision and harness version; record them in the
  result metadata.
- Every trial gets a fresh container/session.
- Secrets are injected through the environment. Never commit keys, gateway
  addresses, or account details; Harbor uses `--env-file`, and the xylitol
  adapter writes a 0600 `secret.env`; `config.yaml` only references
  `{{ secret.MAGPIE_GATEWAY_KEY }}`.

## Phases

| phase | scope |
|---|---|
| Phase 1 | Harbor + xylitol single task |
| Phase 2 | DeepSWE single task + static musl |
| Phase 3 | xylitol vs Pi, DeepSWE 5-task typical set, N=3 |
| Phase 4 | golden + replay + session snapshot |
| Phase 5 | CI / Langfuse bypass / cloud |

SWE-bench Verified is not part of the benchmark set: its upstream images are
`linux/amd64` only, and Harbor's Compose build asks for the host platform. The
remaining benchmarks are Harbor tasks; see `research/baseline-2026-11.md`.

## Usage

```bash
cd tools/eval
uv run xylitol-eval doctor --config examples/eval-terminal-bench.yaml --lock model-lock.yaml
uv run --extra dev pytest -m "not integration"
```

Or from the repo root:

```bash
just eval-test
just eval-smoke --check   # no model call; prerequisites only
just eval-smoke           # one short xylitol trial; expects reward 1
just eval-typical xylitol # 5 DeepSWE tasks, N=3
just eval-typical pi      # same 5 tasks with preinstalled Pi
just eval-typical-both    # xylitol then Pi
```

## Smoke

`just eval-smoke` is the maintenance canary: one repo-local Harbor task,
one xylitol trial, 8 turns, 120s wall. It checks the binary, Harbor wiring,
secret injection, proxy, model call, verifier and result parsing. It is not a
score and does not replace the weekly baseline.

`just eval-smoke --check` validates prerequisites without a model call:
Harbor version, binary SHA-256, env file, Docker, fixture lint. If the model
quota is exhausted, the smoke reports `smoke blocked: model quota/rate limit`
instead of a framework failure.

The fixture lives in `tools/eval/fixtures/smoke-task/`; keep it small and
deterministic. A smoke failure means fix the framework before a full run.

## Typical values

A single run or N=2 is a pilot. For cross-harness comparison, use N=3 or more:

- Primary set: 5 DeepSWE tasks (`abs-module-cache-flags`,
  `actionlint-action-pinning-lint`, `adaptix-name-mapping-aliases`,
  `anko-typed-variable-bindings`, `arcane-drift-detection-baselines`).
- TB4 10 is a hard subset; keep xylitol N=2 records only, do not run Pi there.
- Same budget: 1800s wall / 80 turns for xylitol and Pi.
- Same environment: git identity, CA bundle, gateway allowlist; Pi disables
  web search (xylitol has no web tool).
- Report per-task pass@1, overall mean, flip rate and cross-scaffold gap; drop
  invalid trials (quota, provider error, platform mismatch) and rerun.
- Pi on DeepSWE `no-network` tasks uses preinstalled images; see below.

## Images

Keep only what the 5-task typical set needs:

- 5 original DeepSWE task images (xylitol);
- 5 `xylitol-eval/agents:<task>` Pi preinstalled images;
- the amd64 egress sidecar.

TB4's 20 images and the 5 unused DeepSWE images are removed. Re-pull them only
when running TB4 or the full 10-task DeepSWE set.

## Harbor

Install Harbor in a temporary venv (do not add it to the repo lock):

```bash
uv venv /tmp/harbor-venv --python 3.12
uv pip install --python /tmp/harbor-venv/bin/python 'harbor>=0.14,<1.0'
```

Put magpie credentials in a 0600 env file:

```bash
umask 077
printf 'MAGPIE_GATEWAY_URL=%s\nMAGPIE_GATEWAY_KEY=%s\n' "$MAGPIE_GATEWAY_URL" "$MAGPIE_GATEWAY_KEY" > /tmp/magpie-harbor.env
```

Run one Terminal-Bench 4.0 task with the custom xylitol agent:

```bash
PYTHONPATH=tools/eval/harbor harbor run \
  -p /tmp/tb4/terminal-bench/layout-config-recreation \
  -a xylitol_agent:XylitolAgent \
  -m openai/group/glm-5.3-flash \
  --ak binary=target/eval-bin/xylitol-linux-x86_64-musl \
  --ak max_turns=8 --ak wall_seconds=600 \
  --base-url "$MAGPIE_GATEWAY_URL/v1" \
  --api-key-name MAGPIE_GATEWAY_KEY \
  --api-format openai_responses \
  --env-file /tmp/magpie-harbor.env \
  -n 1 -y
```

Docker Hub may be unreachable. Locally force the legacy builder:

```bash
DOCKER_BUILDKIT=0 COMPOSE_DOCKER_CLI_BUILD=0 harbor run ...
```

### Local proxy wrapper (not CI)

`tools/eval/scripts/harbor_local.sh` wraps `harbor run` for local runs:

- rewrites loopback proxy URLs to `host.docker.internal`;
- appends `NO_PROXY` for the magpie gateway host;
- refuses proxy URLs containing credentials (they would land in argv);
- adds `--allow-agent-host` for the gateway host (so `no-network` tasks can
  reach the model) and `host.docker.internal`;
- caps `-n` / `--n-concurrent` at 3 (default 3, rejects larger values).

Use it when Pi installs need network. Do not wire it into CI; eval CI is not
enabled.

Pi on DeepSWE `no-network` tasks uses preinstalled images:

```bash
PYTHONPATH=tools/eval/harbor harbor run \
  -p /tmp/deep-swe-agents/<task> \
  -a preinstalled_agents:PiPreinstalled \
  -m openai/group/glm-5.3-flash --effort high \
  --ak model_api=openai-responses --ak max_turns=80 \
  --ak disable_web_search=true --agent-timeout-multiplier 0.1666666667 \
  --ae GIT_AUTHOR_NAME=eval --ae GIT_AUTHOR_EMAIL=eval@example.com \
  --ae GIT_COMMITTER_NAME=eval --ae GIT_COMMITTER_EMAIL=eval@example.com \
  --base-url "$MAGPIE_GATEWAY_URL/v1" \
  --api-key-name MAGPIE_GATEWAY_KEY --api-format openai_responses \
  --env-file /tmp/magpie-harbor.env -k 3 -n 1 -y
```

The preinstalled images are `xylitol-eval/agents:<task>`; task copies live in
`/tmp/deep-swe-agents/<task>` and only change `[environment].docker_image`.
Keep the 5 original DeepSWE images and these 5 Pi images; remove TB4 images and
unused DeepSWE images when not running them.

DeepSWE v1.1 single task (Harbor 0.25 runs schema 1.3 + separate verifier):

```bash
harbor run -p /tmp/deep-swe/tasks/httpx-multipart-response-parsing \
  -a xylitol_agent:XylitolAgent -m openai/group/glm-5.3-flash \
  --ak binary=target/eval-bin/xylitol-linux-x86_64-musl ...
```

DeepSWE `no-network` tasks build a Harbor egress sidecar from `gogost/gost`.
When Docker Hub metadata is unreachable, prebuild/tag
`harbor-prebuilt:harbor-docker-egress-control-sidecar--e9f39ce1643ee690` from a
local `gogost/gost` image, then rerun.

Pi installs Node via nvm; behind a host proxy pass
`--ae HTTPS_PROXY=http://host.docker.internal:20171` (and `HTTP_PROXY`).
DeepSWE `no-network` tasks cannot reach nvm/GitHub; use the preinstalled images
above instead of the in-container install.

## Model lock

`model-lock.yaml` pins the model id, API format, context/output limits,
thinking levels, and `smoke` / `compare` profiles. Validate an eval config:

```bash
cd tools/eval
uv run xylitol-eval doctor --config examples/eval-terminal-bench.yaml --lock model-lock.yaml
```

Harbor runs should use the same model id and `--ak max_turns/wall_seconds` from
the selected profile.

## Config

`examples/eval-terminal-bench.yaml` is the active legacy config example. The
Harbor path takes task paths directly and does not use this config.

Required fields: `version`, `phase`, `dataset.name`, `dataset.revision`,
`model.alias`, `agent.binary`.

`dataset.name` can be a Hugging Face name, local `.parquet`, or local
`.jsonl`/`.json`. HF datasets cache under `target/eval-cache/`.

Use the magpie catalog id as the alias (for example
`group/glm-5.3-flash`); do not invent aliases. The model key and base URL come
from `MAGPIE_GATEWAY_KEY` and `MAGPIE_GATEWAY_URL`.

`model.provider: fake` emits a fake-provider config with no network call.

## Outputs

Harbor: `jobs/<job-name>/` with `result.json`, per-trial `agent/`,
`verifier/`, `trial.log`, and task rewards.

## Platform

The remaining Harbor images are single-platform `linux/amd64`. They run:

- natively on Linux x86_64;
- on macOS arm64 through Docker Desktop's amd64 emulation (Rosetta/QEMU).

Linux arm64 needs amd64 emulation (binfmt/qemu) and is not a supported default.

## Local Linux binary

```bash
source <your-proxy-helper>
tools/eval/scripts/build_linux_musl.sh
```

Output: `target/eval-bin/xylitol-linux-x86_64-musl`; verified statically linked.
The manual `.github/workflows/eval-bin.yaml` builds it.
