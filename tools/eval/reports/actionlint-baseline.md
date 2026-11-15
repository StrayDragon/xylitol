# Actionlint single-task baseline (2026-11-15)

- task: `actionlint-action-pinning-lint`
- scaffold: xylitol
- N: 3
- model: `group/glm-5.3-flash` (openai-responses, thinking high)
- commit: `a19aeb0`
- binary SHA-256: `b92eeade5c14425cb635ae39bc1e53fcf5c36c83b1dcc894fa4dfa0ccb130bab`
- Harbor: `0.25.0`
- dataset: `deep-swe/tasks` v1.1
- profile: compare (max_steps_per_turn 80, wall_seconds 1800)
- run: `jobs/one-actionlint-action-pinning-lint`
- result: pass@1 `0.000`; F2P `0/55`; P2P `145/145`; partial `0.725`; 0 exceptions; 3 trials.

> 3 个 trial 都在 1800s agent timeout 处结束，未提交 commit；collect hook 取到空 patch。
> 这份 baseline 是回归 canary，不是能力结论。全套 DeepSWE 10 + TB4 10 留到 1.0.0 前另开 change。

# Harbor eval summary

## Per scaffold

| benchmark | scaffold | tasks | trials | pass@1 | flip rate | errors | wall (s) | cost |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| deepswe | xylitol | 1 | 3 | 0.000 | 0.000 | 0 | 1577.3 | n/a |

## Per task

| benchmark | scaffold | task | runs | rewards | pass@1 | flip |
|---|---|---|---:|---|---:|---|
| deepswe | xylitol | actionlint-action-pinning-lint | 3 | 0, 0, 0 | 0.000 | no |

## Cross-scaffold

No benchmark has two or more scaffolds.
## 复跑

```bash
just eval-one
python3 tools/eval/scripts/summarize_harbor.py \
  --job deepswe:xylitol=jobs/one-actionlint-action-pinning-lint \
  --out tools/eval/reports/actionlint-baseline.md \
  --json tools/eval/reports/actionlint-baseline.json
```
