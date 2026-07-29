# Design: c1700-add-otel-compaction-spans

## 问题

| 通道 | 现状 | 缺口 |
|---|---|---|
| `XyEvent::CompactionStart/End` | TUI/print 生命周期 | 不进 OTLP |
| fastrace 过程树 | turn / iteration / llm / tool / token.estimate | **无** compact 节点 |
| Langfuse | generation / agent / tool | compact 不可见 |

目标：compact **作为有时长的 observation（type=`span`）** 进入同一时间线，不另起导出栈。

## 目标树形

```text
agent.turn                          # 用户触发 run（若存在）
  ├─ … iteration / llm / tool …
  └─ agent.compaction               # NEW；reason=threshold|overflow|manual
       └─ llm.request               # 摘要调用（尽量挂此 parent）
            (+ token.estimate 等仍可挂 turn)

# 无活跃 turn（如纯手动 /session-compact 且不在 run 内）：
agent.compaction                    # 独立根 + langfuse.session.id
  └─ llm.request?
```

## 决策

| 项 | 选择 | 理由 |
|---|---|---|
| 导出名 | `agent.compaction` | 产品词汇；扩展 otel12，非 `react.*` / 厂商私名 |
| Langfuse type | **`span`** | 有起止时长；非 generation；非点事件 `event` |
| 与 XyEvent | **双通道并存** | Event 管面；span 管观测；不互相替代 |
| Parent | turn 优先，否则独立根 + session | 对齐 otel13 / `token.estimate` |
| 挂载点 | `CompactionOrchestrator` 实际执行路径（force + auto） | Start→工作→End 包住同一 span 生命周期 |
| 摘要 LLM parent | compact span 存活期间设 obs parent（iteration 之上或旁路） | 使 generation 落在 compact 下；失败则降级挂 turn |
| I/O | 默认不写 observation input/output | 摘要可能含敏感上下文；沿用 observation_io 显式档哲学，本 change **不**新增 compact I/O 键 |
| 闸 | `provider_trace_active` | 与 ipt2 / 既有低频 span 一致 |

## 属性（最小集）

| 属性 | 何时 | 说明 |
|---|---|---|
| `langfuse.observation.type=span` | start | + session props |
| `reason` | start | `manual` \| `threshold` \| `overflow` |
| `will_retry` | end（若适用） | 对齐 CompactionEnd |
| `aborted` / status | end | 失败时 `langfuse.observation.level` / `status_message` 可选 |
| `error` / `error_message` | end 失败 | 短文案；勿倾倒整段摘要 |

## 测试 seam（已确认）

| Seam | 覆盖 |
|---|---|
| CollectingReporter 单测（`agent/runtime/obs` 或 compaction 旁路 helper） | 闸开：`agent.compaction` 挂 turn；闸关 noop；reason；无 turn 时独立根 + session |
| live `infra-otel` / `infra-observability` 合约 | otel12/otel8 扩展 + 新 req；ipt4 词汇 |
| **不**扩 domain-compaction XyEvent BDD | Start/End 已有 |

`infra-otel.feature` 中多数场景与既有 otel1–17 一样作 Partitioned GWT SSOT；可执行断言以单测为主（对齐 otel18 策略）。

## 非目标

- 改 wire / 触发公式 / TUI%
- compact 专用 observation_io 配置键
- roadmap M5 子进程出站
- 修复以外的 test-bdd 行为（仅 valid_scope 路径）

## 顺带：test-bdd valid_scope

`tests/bdd.rs` 已不存在；真源为 `tests/bdd/`（含 `main.rs`）。更新 `valid_scope` 与 tb3 措辞中的路径引用，不改 BDD 运行方式。
