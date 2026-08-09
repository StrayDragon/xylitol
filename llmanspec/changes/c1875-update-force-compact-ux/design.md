# Design: c1875 force compact 失败文案

## 决策摘要

| 项 | 决定 |
|---|---|
| 产品 | **A — 仅文案**；不强制再压；instructions 与 pi 同构不改 |
| `Already compacted` | **保留** |
| `session too small` | **替换**（不叠旧串） |
| TUI 失败块 | **本波不做**（仍 Start→End+notice） |

## 失败串 SSOT（用户可见 / prepare 返回）

| 条件 | 新串（exact） | 备注 |
|---|---|---|
| leaf 末条为 Compaction | `Already compacted` | 不变 |
| leaf 为空 | `Nothing to compact (empty session)` | 真空；不再用 too small |
| 其余 prepare 拒（cut 后无可摘要消息 / first_kept 无 id 等） | `Nothing to compact (no summarizable history beyond keep window)` | 覆盖痛点会话；**偏离** pi 同文 `session too small` |

前缀仍含 `Nothing to compact`，既有 BDD step（`contains("Nothing to compact")`）可少改。

TUI：`/session-compact failed: {e}` 透传，无第二套文案表。

## 行为不变

```text
/session-compact [instructions?]
  → force（不过 reserve）
  → prepare_compaction
       Err → 新/旧失败串（见上）；无摘要 LLM；body 不变
       Ok  → generate_summary(+ Additional focus?) → CompactionEntry → body 重置
```

- Auto prepare 失败仍静默。
- instructions 仍只在 Ok 路径追加 `Additional focus:`。

## Specs 着陆

- **capability**：`domain-compaction`（修订 `c17`：错误串 MUST 用上表；MUST NOT 再以 `session too small` 作为有上下文主动 force 的用户可见主串）。
- `domain-compaction.feature`：`@req:c17` 场景措辞对齐（仍可断言 `Nothing to compact` 前缀 + 禁止误报）；不必为纯文案单开新 req，除非 validate 要求拆分。
- `app-tui-commands`：**不改**（透传 Driver 错误）。
- `infra-otel` otel19：仍写「无可摘要历史 / Already compacted」即可，不必钉死旧英文。

## PI 偏离

- `src/app/tui/PI_DELTAS.md` 增一行：force prepare 无可摘要历史时用户可见串 **偏离** pi `Nothing to compact (session too small)` → 上表 keep-window / empty 串；闸语义仍同构。

## 测试 seam

复用既有边界，不新造 harness：

| Seam | 覆盖 |
|---|---|
| `prepare_compaction` 单测 | 空 / tip=Compaction / keep 窗无可摘要 → 新串 exact |
| BDD `domain-compaction` `@req:c17` | force 不过 reserve；误报 Nothing to compact 护栏；step 认前缀 |
| `steps_agent_session_extra` slash→compact | 仍认 `Nothing to compact` / `Already compacted` |

## 非目标

强制再压、跳过 prepare、改 keep_recent/auto 公式、改 Additional focus、观测栈、短名 `/compact`、TUI 仅 notice。
