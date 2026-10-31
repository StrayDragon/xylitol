# xylitol-eval

Headless SWE-bench eval harness for xylitol. Developer tooling, not a product
feature.

## Contract

- Scores come only from the official grader (SWE-bench `run_evaluation` or
  Harbor verifier). No self-made judge.
- Drive the agent only through `xylitol print` (headless). Do not drive the TUI
  or send keystrokes.
- Pin the dataset revision and record it in `run.json`.
- Every instance gets its own Docker container and a fresh session.
- Secrets are injected through the environment. Never commit keys, gateway
  addresses, or account details.

## Phases

| phase | scope |
|---|---|
| m0 | Docker + Verified 5 + official grader |
| m1 | wall clock, max_turns, config lock, no ask |
| m2 | Verified 50 + mini-SWE-agent baseline |
| m3 | Harbor + Terminal-Bench |
| m4 | failure -> Dataset; small CI set |
| m5 | cloud full run |

Phase 0 (config, dry-run, doctor, report schema) and Phase 1 (Docker run,
patch collection, official grader) are implemented. Later phases are
scaffolded by the config schema and `ports.py`.

## Usage

```bash
cd tools/eval
uv run xylitol-eval dry-run --config examples/eval-swebench-mini.yaml
uv sync --extra harness
uv run --extra harness xylitol-eval run --config examples/eval-swebench-mini.yaml
uv run xylitol-eval report --run-dir ../../target/eval-runs/<run_id>
uv run --extra dev pytest -m "not integration"
```

Or from the repo root:

```bash
just eval-test
just eval-dryrun
```

`just eval-smoke` runs the real config and needs Docker plus model
credentials. Prefer a zero-cost `provider: fake` config for container smoke
tests.

## Config

See `examples/eval-swebench-mini.yaml`. Required fields: `version`, `phase`,
`dataset.name`, `dataset.revision`, `model.alias`, `agent.binary`.

`dataset.name` can be a Hugging Face dataset name, a local `.parquet` file, or
a local `.jsonl`/`.json` file. HF datasets are cached under
`target/eval-cache/`.

The agent config is generated at run time. Use the magpie catalog id as the
alias (for example `zhipu/glm-5.3`); do not invent aliases. The model key and
base URL come from `MAGPIE_GATEWAY_KEY` and `MAGPIE_GATEWAY_URL`.

`model.provider: fake` emits a fake-provider config with no network call. Use
it to validate container orchestration, patch collection and logging without
spending tokens.

`grader.run_id` must be unique per run. SWE-bench caches per-instance results
by `run_id`; reusing it returns stale outcomes.

## Outputs

`target/eval-runs/<run_id>/`:

```text
run.json
summary.md
instances.jsonl
preds.jsonl
logs/
reports/
```

`run.json` records the binary SHA-256, git commit, config SHA-256, dataset
revision, harness version, per-instance outcome and summary.

## Local Linux binary

Because the repo is not pushed until review, CI is not available. Build the
Linux binary locally in Docker (glibc 2.35, matching the SWE-bench image):

```bash
source <your-proxy-helper>   # for example: proxing on
tools/eval/scripts/build_linux.sh
```

The script defaults to an already-pulled SWE-bench image when Docker Hub is
unreachable. Override with `XYLITOL_EVAL_BUILD_IMAGE=<image>`. The manual
`.github/workflows/eval-bin.yaml` reuses the same cargo command.

The binary is written to
`target/eval-bin/xylitol-linux-x86_64-glibc`; its SHA-256 is recorded in
`run.json`.
