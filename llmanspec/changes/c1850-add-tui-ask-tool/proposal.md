---
depends_on: []
---

# TUI-only 内置工具 `ask`（澄清 / 分叉）

## Why

Agent 在需求不清或实现分叉时需要结构化问用户，但：

- 产品 `EditorSlot::Choice` 仍是 stub；Trust 闸的 Single ChoicePrompt 不能复用为会话中途澄清。
- 拆成多个 Single/Multi 工具会重复占 tools 上下文。
- 「不想答」应是 **skip 成功结构化结果**，不是 tool error / 假超时。

包侧 ChoicePrompt + `agent_demo` Ask 脸已打磨（rail、说明可选、Review 未答二次确认）；本 change 把合约与产品接线落地。

## What Changes

- **一个**内置工具 `ask`（UI 名 **Ask**），**仅 TUI** 注册；Print MUST NOT 暴露。
- 包装器：1×Single → 单选；1×Multi → 多选；≥2 → Tabs + Review。
- Skip / Esc → `{ status: skipped }`；全答提交 → `{ status: answered, answers }`；MVP 无超时。
- Review 有未答 tab：首次 Enter 提示缺口；再 Enter 确认 Skip；← 取消二次确认。
- `option.description` / `recommended`：**对人可选**；agent MAY 省略 description。
- 产品 scrollback Ask 块：固定左轨（waiting/answered/skipped 语义色）+ 人话摘要；MUST NOT `tool-*-bg` / raw JSON 默认洗底。
- 解冻产品 Choice 槽；Driver/TUI 等答缝（进程内）；桥接特殊处理 Ask ≠ 通用 Tool 块。

## Capabilities

- `app-tui-ask`（新）— 产品 Ask 槽、仅 TUI 装配、scrollback 人话、rail
- `package-tui-choice-prompt`（扩）— Skip 成功语义、Review 二次确认、说明可选、rail/选中态
- `agent-tools`（扩）— 内置 `ask` schema / 执行与结果；Print 不注册

## 测试缝（apply 用；propose 已对齐）

| 缝 | 断言 |
|---|---|
| 包 `ChoicePrompt` 单测 | Skip / Review 未答二次确认 / description 缺省不空栏 |
| `agent_demo` harness `ask-*` | 槽打开、Skip、假工具回灌摘要 |
| 产品 BDD `app-tui-ask` | TUI 注册 ask；槽打开；skip/answered 回灌（复用现有 TUI/BDD harness） |
| `app-tui-trust` | 本 change **不改** Trust 合约；回归绿 |

## Impact

- **Trust 闸**：不变（仍 bootstrap 前 Single）。
- **Print / Server**：不注册 `ask`；WS `AnswerQuestion` 不在本 change 接线。
- **危险确认**：另案。
- **跨面**：语义属「agent 问用户」；今日仅 TUI；Web 后置勿另起冲突故事。

## Decisions

| 项 | 定稿 |
|---|---|
| change id | `c1850-add-tui-ask-tool` |
| 工具名 | `ask`；UI **Ask** |
| 可见性 | 仅 TUI |
| 等答缝 | 进程内 oneshot（TUI host）；不对齐 wire `AnswerQuestion`（本 change） |
| 产品轨 | **仅 rail**；demo 可 `/entry-style` wash 对照 |
| description | agent MAY 省略 |
| Review 未答 | Enter ×2 → Skip |

## Out of scope

- 超时 auto-skip / 假超时串
- IDE 真右侧 DOM 面板（TUI 同宽分栏即可）
- Web / Print ask
- Trust / 危险确认复用 ask

## Further Notes

- 视觉 SSOT：[`src/app/tui/design/ask.md`](../../../src/app/tui/design/ask.md)
- 静图：`playground/?slot=ask`
- Skip：[`docs/research/ask-tool-skip-semantics-2026.md`](../../../docs/research/ask-tool-skip-semantics-2026.md)
- UI 景观：[`docs/research/ask-ui-ux-landscape-2026.md`](../../../docs/research/ask-ui-ux-landscape-2026.md)
- 前草案：`tui-only-builtin-ask-tool-clarify-decision-questionnaire`（已提升为本 id）
