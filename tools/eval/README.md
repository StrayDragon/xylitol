# xylitol-eval

Headless agent eval harness for xylitol. Developer tooling, not a product
feature.

## Contract

- Scores come only from the official verifier/grader: Harbor task verifier or
  SWE-bench `run_evaluation`. No self-made judge.
- Drive the agent only through `xylitol print` (headless). Do not drive the TUI
  or send keystrokes.
- Pin the model, dataset revision and harness version; record them in the
  result metadata.
- Every trial gets a fresh container/session.
- Secrets are injected through the environment. Never commit keys, gateway
  addresses, or account details; Harbor uses `--env-file`, and the xylitol
  adapter writes the key into a 0600 container config instead of `exec -e`
  argv.

## Phases

| phase | scope |
|---|---|
| Phase 1 | Harbor + xylitol single task |
| Phase 2 | TB4.0 / DeepSWE single task + static musl |
| Phase 3 | xylitol vs Pi vs mini-SWE, 25/10/10, N=3 |
| Phase 4 | golden + replay + session snapshot |
| Phase 5 | CI / Langfuse bypass / cloud |

The old official SWE-bench runner remains as a fallback/parity path, not the
primary capability gate.

## Usage

```bash
cd tools/eval
uv run xylitol-eval dry-run --config examples/eval-swebench-mini.yaml
uv sync --extra harness
uv run --extra harness xylitol-eval run --config examples/eval-swebench-mini.yaml
uv run xylitol-eval doctor --config examples/eval-swebench-mini.yaml --lock model-lock.yaml
uv run --extra dev pytest -m "not integration"
```

Or from the repo root:

```bash
just eval-test
just eval-dryrun
just eval-harbor   # needs MAGPIE_GATEWAY_URL/KEY and /tmp/harbor-venv
```

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

Run one SWE-bench Verified task with the custom xylitol agent:

```bash
PYTHONPATH=tools/eval/harbor harbor run \
  -d swe-bench/swe-bench-verified \
  -i '*astropy__astropy-12907*' \
  -l 1 \
  -a xylitol_agent:XylitolAgent \
  -m openai/group/glm-5.3-flash \
  --ak binary=target/eval-bin/xylitol-linux-x86_64-glibc \
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

Phase 3 adds Pi and mini-SWE with the same model, dataset and verifier:

```bash
harbor run -d swe-bench/swe-bench-verified -l 25 -k 3 \
  -a pi -m openai/group/glm-5.3-flash --effort high \
  --base-url "$MAGPIE_GATEWAY_URL/v1" \
  --api-key-name MAGPIE_GATEWAY_KEY --api-format openai_responses \
  --ak model_api=openai-responses --env-file /tmp/magpie-harbor.env
```

```bash
harbor run -d terminal-bench/terminal-bench@4.0.0 -l 10 -k 3 \
  -a mini-swe-agent -m openai/group/glm-5.3-flash --ak version=2.1.0 ...
```

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
uv run xylitol-eval doctor --config examples/eval-swebench-mini.yaml --lock model-lock.yaml
```

Harbor runs should use the same model id and `--ak max_turns/wall_seconds` from
the selected profile.

## Config / fallback

`examples/eval-swebench-mini.yaml` is the official SWE-bench fallback. Required
fields: `version`, `phase`, `dataset.name`, `dataset.revision`,
`model.alias`, `agent.binary`.

`dataset.name` can be a Hugging Face name, local `.parquet`, or local
`.jsonl`/`.json`. HF datasets cache under `target/eval-cache/`.

Use the magpie catalog id as the alias (for example
`group/glm-5.3-flash`); do not invent aliases. The model key and base URL come
from `MAGPIE_GATEWAY_KEY` and `MAGPIE_GATEWAY_URL`.

`model.provider: fake` emits a fake-provider config with no network call.

`grader.run_id` must be unique per run; SWE-bench caches by `run_id`.

## Outputs

Official fallback: `target/eval-runs/<run_id>/` with `run.json`,
`summary.md`, `instances.jsonl`, `preds.jsonl`, `logs/`, `reports/`.

Harbor: `jobs/<job-name>/` with `result.json`, per-trial `agent/`,
`verifier/`, `trial.log`, and task rewards.

## Local Linux binary

```bash
source <your-proxy-helper>   # for example: proxing on
tools/eval/scripts/build_linux.sh
```

The script defaults to an already-pulled SWE-bench image when Docker Hub is
unreachable. Override with `XYLITOL_EVAL_BUILD_IMAGE=<image>`. The manual
`.github/workflows/eval-bin.yaml` reuses the same cargo command.

Binary: `target/eval-bin/xylitol-linux-x86_64-glibc`; SHA-256 is recorded in
`run.json`.

Static musl build for non-Ubuntu Harbor tasks:

```bash
source <your-proxy-helper>
tools/eval/scripts/build_linux_musl.sh
```

Output: `target/eval-bin/xylitol-linux-x86_64-musl`; verified statically linked.
