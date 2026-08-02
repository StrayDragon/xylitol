---
version: "0.4"
name: "ask"
description: "TUI-only builtin tool ask — clarify/decision questionnaire via ChoicePrompt wrapper faces."
tokens_from: "../DESIGN.md"
components:
  ask-caption:
    textColor: "{colors.accent}"
  ask-prompt:
    textColor: "{colors.on-surface}"
  ask-selected:
    textColor: "{colors.accent}"
  ask-tab-active:
    textColor: "{colors.accent}"
  ask-tab-idle:
    textColor: "{colors.muted}"
  ask-skip:
    textColor: "{colors.accent}"
  ask-hint:
    textColor: "{colors.muted}"
  ask-summary:
    textColor: "{colors.on-surface}"
  ask-summary-muted:
    textColor: "{colors.muted}"
  ask-rail-waiting:
    textColor: "{colors.accent}"
  ask-rail-answered:
    textColor: "{colors.success}"
  ask-rail-skipped:
    textColor: "{colors.muted}"
  ask-desc:
    textColor: "{colors.muted}"
---

# Ask（内置工具 `ask`）

> Token：`{colors.*}` → [`../DESIGN.md`](../DESIGN.md)。
> 静图：[`playground/`](./playground/) `?slot=ask`。
> Skip 语义：[`docs/research/ask-tool-skip-semantics-2026.md`](../../../docs/research/ask-tool-skip-semantics-2026.md)。
> UI 景观：[`docs/research/ask-ui-ux-landscape-2026.md`](../../../docs/research/ask-ui-ux-landscape-2026.md)。
> Trust 闸 **不是**本工具：[`trust-prompt.md`](./trust-prompt.md)。

## 产品意图

Agent 在 **计划澄清 / 需求不清 / 实现分叉** 时调用 **一个** 内置工具 `ask`。

| MUST | 禁止 |
|---|---|
| 仅 **TUI** 注册 | Print 暴露 `ask` |
| 一工具三脸包装器 | 拆成多个 Single/Multi 工具 |
| 人对人话；JSON 只给模型 | scrollback 默认 raw JSON / `tool-*-bg` 洗底 |
| Ask 块 **固定左边轨** | 与 Read/Bash 绿条洗底混用 |

## 双受众

| 受众 | 载体 |
|---|---|
| LLM | `ask` tool JSON |
| 人 | editor 槽问卷 + scrollback 摘要（轨 + 人话） |

## 视觉 MUST（editor 槽）

对齐 Claude AskUserQuestion：`label` 必有；**description 对人可选增强**；宽终端可右侧说明栏。

| 字段 | 对 agent / tool JSON | 对人 UI |
|---|---|---|
| `label` | MUST | 选项主文 |
| `description` | **MAY 省略** | 有则展示易懂例子；无则不占行、不开空「说明」栏 |
| `recommended` | MAY | 有则尾随 ` · 推荐` |

1. **左边轨（产品）**：问卷每一行经 `paint_left_rail_line`，轨色默认 `{colors.accent}`（与 waiting 一致）。**产品 `src/app/tui` 仅 rail**——无 wash 切换。Demo `agent_demo` 可用 `/entry-style rail|wash` 对照 pi 整行洗底。
2. **标题「Ask」**：caption 用 `{colors.accent}`（可粗体）。
3. **选中态**：accent + **粗体**（`→` / `[x]` 与 label）；**MUST NOT** 默认整行 reverse 洗屏。
4. **提问 ↔ 选项**：prompt 与选项列表之间 **MUST** 空一行（防挤）。
5. **说明 / 易懂例子**（`option.description`，可选）：
   - 全部缺省 → 单栏选项，无说明行、无右侧空栏。
   - **窄宽**（&lt; ~88）且有 description：仅在**当前聚焦**选项下方展示 muted 说明。
   - **宽屏**（≥ ~88）且至少一项有 description：左列选项 · 右列「说明」面板。
6. Tabs / Review / Skip 底栏：同前；Skip accent。**多题 Review 有未答**：Enter 二次确认 Skip（见下表）。

## 视觉 MUST（scrollback Ask 块）

| phase | 轨色 | 摘要例 |
|---|---|---|
| waiting | `{colors.accent}` | `Ask · 等待回答…` |
| answered | `{colors.success}` | `Ask · 已选  最小可运行切片` |
| skipped | `{colors.muted}` | `Ask · 已跳过 · 按已有信息继续` |

- **MUST** 左边轨 + gutter；**MUST NOT** `tool-*-bg` 整行洗底。
- 默认一行摘要；**Alt+E** 展开 `id → labels`（人话）。
- **MUST NOT** 重复 JSON ScrollNotice。

## 包装器脸

| Payload | 脸 |
|---|---|
| 1×Single | 单选 |
| 1×Multi | 多选 |
| ≥2 | Tabs + Review |

Other 默认开。单选无 ●/○。

## Skip / 结果

| 动作 | 结果 |
|---|---|
| Skip / Esc | `{ status: skipped }` 成功 |
| 提交（全答） | `{ status: answered, answers }` |
| **Review + 有未答 tab**：首次 Enter | 提示「有 tab 未填写…」；**不**提交、**不** skip |
| 同提示下再 Enter | 确认 Skip（二次确认） |
| ← / 切题 | 取消二次确认，回去填写 |
| Abort 整轮 | 非 skip |
| 超时 | MVP 无 |

## 非目标

Trust 闸 · 危险确认 · Settings · 默认展开 raw JSON · IDE 真·右侧 DOM 面板（TUI 用同宽分栏模拟）

## 落地

1. ~~静图 / ask.md / demo（rail · 说明 · 人话摘要）~~
2. `c1850-add-tui-ask-tool` → Specs landing → 产品接线（`llman-sdd-apply`）

手验：`just demo-tui` → `ask-single`（看说明）· `ask-tool`（看轨色）· `/entry-style wash` 对照 pi 洗底 · 拉宽终端看右侧「说明」。
