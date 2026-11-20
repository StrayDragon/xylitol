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

## Scope

Current baseline is **one representative DeepSWE task**:

- task: `actionlint-action-pinning-lint`
- scaffold: xylitol or Pi
- N=3; 1800s wall / 120 steps
- model: `group/glm-5.3-flash`

The full DeepSWE 10 + TB4 10 sweep is deferred to the 1.0.0 release cycle.
`just eval-smoke` remains the framework canary.

## Usage

```bash
cd tools/eval
uv run xylitol-eval doctor --config examples/eval-one.yaml --lock model-lock.yaml
uv run --extra dev pytest -m "not integration"
```

Or from the repo root:

```bash
just eval-test
just eval-smoke --check   # no model call; prerequisites only
just eval-smoke           # one short xylitol trial; expects reward 1
just eval-one xylitol     # actionlint task, xylitol, N=3
just eval-one pi          # actionlint task, Pi preinstalled, N=3
```

## Smoke

`just eval-smoke` is the maintenance canary: one repo-local Harbor task,
one xylitol trial, 8 turns, 120s wall. It checks the binary, Harbor wiring,
secret injection, proxy, model call, verifier and result parsing. It is not a
score and does not replace the baseline.

`just eval-smoke --check` validates prerequisites without a model call:
Harbor version, binary SHA-256, env file, Docker, fixture lint. If the model
quota is exhausted, the smoke reports `smoke blocked: model quota/rate limit`
instead of a framework failure.

The fixture lives in `tools/eval/fixtures/smoke-task/`; keep it small and
deterministic. A smoke failure means fix the framework before a baseline run.

## Single-task baseline

`just eval-one <scaffold>` runs `actionlint-action-pinning-lint` with N=3. Both scaffolds use 120 steps. Report
per-task pass@1, wall, cost and commit/SHA; do not treat one task as a
capability ranking. For a stable signal, rerun with the same commit, lock and
profile.

Latest baseline: `tools/eval/reports/actionlint-baseline.md`.
Historical 5-task results: `llmanspec/changes/archive/2026-11-09-c3006-add-minimal-eval-harness/research/eval-report-2026-11.md`.

## Images

Keep only what the single-task baseline needs:

- one DeepSWE task image (`actionlint-action-pinning-lint`);
- the amd64 egress sidecar.

Pi uses one preinstalled image for the same task; TB4 images are not used.
Pi needs `/tmp/deep-swe-agents/actionlint-action-pinning-lint`; build/prepare it before `just eval-one pi`.
Re-pull them only for the deferred 1.0.0 sweep.

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

Run one DeepSWE task with the custom xylitol agent:

```bash
PYTHONPATH=tools/eval/harbor harbor run \
  -p /tmp/deep-swe/tasks/actionlint-action-pinning-lint \
  -a xylitol_agent:XylitolAgent \
  -m openai/group/glm-5.3-flash \
  --ak binary=target/eval-bin/xylitol-linux-x86_64-musl \
  --ak max_steps_per_turn=120 --ak wall_seconds=1800 \
  --base-url "$MAGPIE_GATEWAY_URL/v1" \
  --api-key-name MAGPIE_GATEWAY_KEY \
  --api-format openai_responses \
  --env-file /tmp/magpie-harbor.env \
  -k 3 -n 3 -y
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

Use it when the task needs network. Do not wire it into CI; eval CI is not
enabled.

## Model lock

`model-lock.yaml` pins the model id, API format, context/output limits,
thinking levels, `smoke` / `compare` profiles, and the single baseline task.
Validate an eval config:

```bash
cd tools/eval
uv run xylitol-eval doctor --config examples/eval-one.yaml --lock model-lock.yaml
```

Harbor runs should use the same model id and `--ak max_steps_per_turn/wall_seconds` from
the selected profile.

## Config

`examples/eval-one.yaml` is the single-task config example. The Harbor path
takes task paths directly and does not use this config.

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
