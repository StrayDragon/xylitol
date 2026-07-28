---
change_id: c1620-add-print-eval-foundations
title: Print eval 基础：trust 旗标、失败 exit、可选 max_turns
status: designed
priority: 1620
depends_on: []
author: agent
branch: sdd/c1620-add-print-eval-foundations
base_sha: 13242412a91d2e3097afbde2704256ceba3f5903
checkpointed: true
checkpoint_sha: 13242412a91d2e3097afbde2704256ceba3f5903
---

# c1620-add-print-eval-foundations

## Why

SWE-bench / Harbor 等社区 harness 以 **headless `xylitol print`** 驱动自主多轮，不经产品 TUI。当前 print 面缺三项会阻塞可靠冒烟：

1. **无 `--trust` / `--no-trust`**（`ce19` 刻意禁止）→ 容器内若有 `.xylitol/`，非交互常 `FallbackNoUi` 跳过项目资源；
2. **agent 失败仍 exit 0** → 编排无法区分「崩了」与「交了空 patch」；
3. **无可选轮次上限** → 与社区 `step_limit` 不对齐，子集跑批可无限烧钱（产品默认仍禁止硬闸，见 ar1/ar16/rc22）。

本 change **只做项目内基础**；Docker / `preds.jsonl` / `just eval-swe-*` / Harbor adapter **不在范围**。

## What Changes

- `xylitol print` 接受 `--trust` / `--no-trust`，经同一 bootstrap `trust_override` 路径生效（与 tui 对齐）；仍 **MUST NOT** 接受 `--list-models`。
- Print 在运行时出现 `XyEvent::Error` 或等价不可恢复失败时，进程 **非 0 退出**；成功完成（含模型自然结束）保持 0。
- 可选配置 `session.max_turns`（正整数）：装配时安装 `should_stop_after_turn`，在完成 N 个 turn 后结束 run；**缺省 / 未配置 = 不安装**（保持 ar24 开放结束）。**MUST NOT** 恢复已移除的 `max_iterations` 旋钮名。
- 更新 `ce19`、`cli-print`、`agent-runtime`、`runtime-config` 合约与可执行场景。

## Capabilities

| Capability | 变更 |
|---|---|
| `cli-entry` | 修订 `ce19`：print 允许 trust 旗标 |
| `cli-print` | 新增失败非 0 exit |
| `agent-runtime` | 可选 max_turns → `should_stop_after_turn`（不改默认） |
| `runtime-config` | `SessionConfig.max_turns`；澄清与 `rc22` 关系 |

## Impact

- **破坏性**：`ce19` 原「print MUST NOT 接受 --trust」改为允许；旧文档/脚本若依赖「print 拒绝 --trust」会变。
- **默认体验**：未配 `max_turns`、未传 trust 时行为与今日一致（除 exit 语义：原先 Error 仍 0 → 变为非 0，属有意修正）。
- **非目标**：autosubmit / fake_user / SWE 编排脚本 / Harbor / TUI 自动驱动。

## Ethics

- risk_level: medium
- prohibited_actions: 恢复产品默认 `max_iterations` 硬闸；以自动 TUI 按键作为 eval 前置
- required_evidence: BDD 或等价单测覆盖 print trust 解析/bootstrap、print Error→非 0、配置 max_turns 触发 AgentEnd；未配置时开放结束
- refusal_contract: 不在本 change 引入外部 Docker/harness 依赖进 `just qa` 默认闸
- escalation_policy: 若发现 exit 码会破坏既有依赖 print 的脚本，先在 proposal/design 钉清语义再 apply
