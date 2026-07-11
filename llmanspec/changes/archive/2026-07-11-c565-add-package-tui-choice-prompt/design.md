# Design — c565-add-package-tui-choice-prompt

## 形态（内联槽，非 overlay）

```
┌ ChoicePrompt ──────────────────────────────────┐
│ [Q1 Scope] [Q2 Priority] [Submit]   ← 多题 Tab │
│ 提示：本轮优先做什么？                          │
│ ○ / [x] 选项 A                                  │
│ ○ / [ ] 选项 B                                  │
│ ○ / [ ] Other…                                  │
│   > 自由输入█          ← Tab 聚焦后才可打字     │
│ ↑↓ · Space(多选) · Enter · Tab→Other · Esc      │
└─────────────────────────────────────────────────┘
```

## 模式

| 模式 | 选择语义 | Enter | 行视觉 |
|---|---|---|---|
| `Single` | 互斥；cursor = 当前答案 | 确认该项并前进/提交 | **无 ●/○**；`→` + 反色（对齐 SelectList） |
| `Multi` | Space 勾选；高亮 ≠ 勾选 | 提交本题已选项 | `[x]` / `[ ]` + `→` |

多题 Tab **可混搭**每题 Single/Multi；Tab 标签：`[Name]` 单选、`[Name+]` 多选、已答加 `✓`。


## 键位（相对 SelectList 的增量）

| 键 | 作用 |
|---|---|
| ↑↓ | 选项间移动（含 Other 行） |
| Space | **仅 Multi**：切换勾选 |
| Enter | Single：选中并前进；Multi：提交本题；Submit 页：完成问卷 |
| Tab | 当前在 Other 行或已允许 Other 时：**聚焦自由输入**；输入中 Tab 失焦回列表 |
| Esc | 取消整份问卷（`cancelled: true`） |

## 命名

- 类型：`ChoicePrompt` / `ChoiceQuestion` / `ChoiceMode::{Single,Multi}` / `ChoiceAnswer`
- 避免 `AskQuestion`（产品工具名）；包侧保持通用名词

## 与既有组件关系

- **复用**：窄宽 clamp、主题闭包、`Input` 作 Other 行编辑器
- **不复用**：不把多选硬塞进 `SelectList`（语义分叉过大）

## Playground

新槽 `ask`（键位接在 Atoms 后）：芯片切换 Single / Multi / Tabs；示意 Other 聚焦态。
