# Agent Eval 进度报告（2026-11-08）

## 一句话结论

在 5 个 DeepSWE task、同模型、同 verifier、N=3 的条件下，xylitol 与 Pi 基本打平：
4/5 个 task 的通过率完全相同；唯一差异是 `arcane`，Pi 有一次 1800s 超时。

## 口径

- 模型：`group/glm-5.3-flash`，同一网关、同一 API。
- 任务：5 个 DeepSWE task，覆盖 Go / Rust / TypeScript / Python 等不同仓库。
- 预算：1800s wall、80 turns；xylitol 与 Pi 相同。
- 采样：每个 task × 每个 scaffold × N=3，共 15 trials/scaffold。
- 评分：只认 Harbor task verifier 的 reward；`partial` 是测试通过比例，不等于 pass。
- 环境：Pi 用预装镜像，DeepSWE `no-network` 下不安装依赖；两边都设 git identity。

## 结果

| task | xylitol | Pi | 差异 |
|---|---:|---:|---:|
| abs-module-cache-flags | 100% (3/3) | 100% (3/3) | 0 |
| actionlint-action-pinning-lint | 33% (1/3) | 33% (1/3) | 0 |
| adaptix-name-mapping-aliases | 0% (0/3) | 0% (0/3) | 0 |
| anko-typed-variable-bindings | 33% (1/3) | 33% (1/3) | 0 |
| arcane-drift-detection-baselines | 67% (2/3) | 33% (1/3) | -33 pp |
| **总体** | **46.7% (7/15)** | **40.0% (6/15)** | **-6.7 pp** |

- 总体差 6.7 个百分点，约等于 1 个 trial。
- 跨 scaffold 差异只出现在 `arcane`；其余 4 个 task 的 pass@1 完全一致。
- Pi 的 `arcane` 有一次 `AgentTimeoutError`（1800s），该 trial 计为失败。
- 两边各有 3/5 个 task 不是 3/3 稳定通过；N=3 下波动明显。

## 怎么读

- `abs-module`：两边都稳定 100%，是“能过”的基线。
- `actionlint`：两边都只过 1/3，说明这个 task 对两边都不稳。
- `adaptix`：两边都是 0/3，但每次 partial 都是 98.4%，属于“只差一个测试点”的近失。
- `anko-typed`：两边都只过 1/3，波动一致。
- `arcane`：xylitol 2/3，Pi 1/3；Pi 的一次超时是唯一实质差异。

因此当前不能说 xylitol 比 Pi 更强；差距落在一次超时和 N=3 噪声范围内。

## 成本与资源

- Pi：15 trials，约 41.9M input tokens、0.49M output tokens、40.9M cache tokens。
- xylitol：结果文件未记录 token 用量；wall time 约 4 小时（5 task 合计）。
- Pi：wall time 约 5 小时 18 分（5 task 合计，含一次 1800s 超时）。
- 镜像：保留 5 个 DeepSWE 原始镜像 + 5 个 Pi 预装镜像；TB4 20 个镜像和未用 DeepSWE 5 个已删除。
- 磁盘：剩余约 61GB。

## 复跑

```bash
just eval-typical xylitol
just eval-typical pi
just eval-typical-both
just eval-smoke --check
```

## 注意

- N=3 只够看趋势，不够排名；一个 trial 就能改变 6.7 个百分点。
- `arcane` 的 Pi 超时是预算耗尽，不是 verifier 错误；如果要区分“能力”与“超时”，可以单独重跑该 trial。
- TB4 已移出当前典型集，只保留 xylitol N=2 历史记录。
- 这份报告是当前快照；下次改动 xylitol 后用同 task、同 N、同预算重跑再比较。
