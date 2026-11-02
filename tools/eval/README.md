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
| Phase 2 | TB4.0 / DeepSWE single task + static musl |
| Phase 3 | xylitol vs Pi vs mini-SWE, TB4.0 10 + DeepSWE 10, N=3 |
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
just eval-harbor   # needs MAGPIE_GATEWAY_URL/KEY and /tmp/harbor-venv
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
- adds `--allow-agent-host host.docker.internal`;
- caps `-n` / `--n-concurrent` at 3 (default 3, rejects larger values).

Use it when Pi/nvm or mini-SWE/uv installs need network. Do not wire it into
CI; eval CI is not enabled.

Phase 3 compares Pi and mini-SWE with the same model, task and verifier:

```bash
harbor run -p /tmp/tb4/terminal-bench/layout-config-recreation -k 3 \
  -a pi -m openai/group/glm-5.3-flash --effort high \
  --base-url "$MAGPIE_GATEWAY_URL/v1" \
  --api-key-name MAGPIE_GATEWAY_KEY --api-format openai_responses \
  --ak model_api=openai-responses --env-file /tmp/magpie-harbor.env
```

```bash
harbor run -p /tmp/tb4/terminal-bench/layout-config-recreation -k 3 \
  -a mini_swe_agent_mcp:MiniSweAgentWithMcp -m openai/group/glm-5.3-flash \
  --ak version=2.1.0 --agent-setup-timeout-multiplier 3 ...
```

The baseline run uses TB4.0 10 + DeepSWE 10, not SWE Verified; see
`research/baseline-2026-11.md` for task ids, model lock and platform notes.

Terminal-Bench 4.0 single task:

```bash
harbor run -p /tmp/tb4/terminal-bench/layout-config-recreation \
  -a xylitol_agent:XylitolAgent -m openai/group/glm-5.3-flash \
  --ak binary=target/eval-bin/xylitol-linux-x86_64-musl \
  --ak max_turns=8 --ak wall_seconds=600 ...
```

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
Mini-SWE's LiteLLM imports the optional `mcp` package, which Harbor's installer
does not add; use `mini_swe_agent_mcp:MiniSweAgentWithMcp`, which installs
`mcp` into the tool venv.

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
