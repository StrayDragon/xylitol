---
version: "alpha"
name: "models-picker"
description: "/model fuzzy list replacing editor slot — not silent cycle (pi-aligned)."
tokens_from: "../DESIGN.md"
components:
  models-title:
    textColor: "{colors.muted}"
  models-selected:
    textColor: "{colors.on-surface}"
  models-match:
    textColor: "{colors.accent}"
  models-hint:
    textColor: "{colors.muted}"
---

# Models picker（`/model`）

> Token 根源：`{colors.*}` → [`../DESIGN.md`](../DESIGN.md)。
> 键位：[`keybindings.md`](./keybindings.md)。静图：[`playground/`](./playground/)「Models」槽。
> 实现 change：**c630-add-app-tui-models-picker**（依赖 **c625** 设计闸）。
> 对齐 pi：命令名 **`/model`**，行为是选择器而非 cycle。

## 产品意图

换模型要**看得见、搜得到**，不要隐式轮换。用户输入 **`/model`**（无参，或补全选中）→ **替换 editor 槽**打开可选列表，支持 **fuzzy** 过滤，Enter 选定，Esc 取消。可选 **`/model <id>`** 直选，不经列表。

## MUST

1. 斜杠命令 **`/model`**（无参）打开选择器；语义经 `protocol::Command` / `dispatch`（`GetAvailableModels` 取列表，选定后 `SetModel`）。
2. **`/model <id>`** MUST 直接 `SetModel`（不打开列表）；未知 id 短错误提示。
3. 选择器 **替换 editor 槽**（与会话树 / ChoicePrompt 同槽模型）；**MUST NOT** 居中 overlay 主路径。
4. 列表展示当前可用模型 id（及可选短标签）；**当前模型**可用 muted 标注或 `*` 前缀，仍保持单行可复制友好。
5. **Fuzzy 过滤**：用户在选择器内键入时过滤列表（对齐包 `SelectList` / demo fuzzy）；无匹配时短提示，**MUST NOT** 崩溃。
6. **Enter** 选定 → `SetModel` → 关槽、还原 editor、footer `model` 字段更新。
7. **Esc** 取消 → 关槽、还原 editor，**MUST NOT** 改模型。
8. **MUST NOT** 再把无参 `/model` 做成静默 cycle；**MUST NOT** 引入 `/models` 作为产品主路径或必推补全。
9. idle 以外（busy）打开 picker：**MUST** 拒绝或排队策略在实现 change 写清；默认建议 idle-only，与双 Esc 树一致（忙碌不开）。

## 形状（固定）

```
… scrollback …
┌ models ─────────────────────────
│ models
│ › ornith-fast *
│   ornith-think
│   other/gpt-mini
└───────────────────────────────
footer  ~/cwd · ornith-think
```

选中行 **整行 reverse**（`.rev`）；标题/filter 行 `{colors.muted}`；Esc 取消不画进槽内墙。无第二套色板。
**选中行内** MUST NOT 再叠 `user`/`accent` 等独立前景（与 [`session-tree.md`](./session-tree.md) 选中对比度同一原则）。

Filter 态标题改为 `filter: orn`；无匹配时：

```
│ filter: zzz
│   No entries found
│   (0/0)
```

## 不做

- `/models` 别名或第二命令。
- 运行时改 YAML/provider 配置（无 Settings 槽）。
- 拉取远端模型市场 / OAuth 订阅面板。
- `/theme` 或主题切换（MVP 固定暗色）。
- pi `Ctrl+Shift+M`（MAY 后续 change）。
- **键入 `/model <前缀>` 的内联自动补全**（后置 **c999**；本文件只约束无参开槽 + `/model <id>` 直选）。

## 验收指针

- playground「Models」槽肉眼对照本文件形状。
- 产品 harness：`/model` → 过滤 → Enter → footer model 变；Esc 不变；无参不再 cycle。
