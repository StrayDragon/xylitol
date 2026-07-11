# src/app/tui/design/

本目录 = 产品 TUI **组件级视觉 MUST**（`*.md`）+ **人类用** DESIGN playground。

Token / 全局 Overview SSOT：上一级 [`../DESIGN.md`](../DESIGN.md)。包引擎边界：`packages/xylitol-tui/AGENTS.md`。本面接线边界：[`../AGENTS.md`](../AGENTS.md)。

## 给人 / 给 agent

| 受众 | 读什么 |
|---|---|
| **人类** | `*.md` MUST；需要快速看色与层次时打开 [`playground/`](./playground/)（浏览器交互：平铺 / 专注 / 切换） |
| **Agent** | 默认读 `DESIGN.md` + 相关 `design/<comp>.md`。**默认忽略** `playground/`（HTML/CSS/JS 预览壳，非运行时、非规范正文） |

Agent **仅在**下列情况读 playground：

1. 人类明确指定路径（如「看 `design/playground`」），或
2. 人类在消息里 **引用 / 粘贴** playground 片段，用来对齐「当前视觉意图」

不要把 playground 当实现真值源；落地仍以 `DESIGN.md` / `design/*.md` MUST + `packages/xylitol-tui` 测试为准。

## 硬约束

- 子文档 MUST 声明 `tokens_from: "../DESIGN.md"`；MUST NOT 另立冲突 hex。
- playground 色值 MUST 来自 `DESIGN.md` frontmatter。改 token 后跑：
  `python3 src/app/tui/design/playground/sync_tokens.py`（生成 `tokens.css` / `tokens.js`；见 playground README）。
- playground 是**人类可交互设计图**（典型态 + 动态），不是实现真值源；落地仍以 MUST + 包测试为准。
- playground **不是** 产品 TUI，**不是** `xylitol-tui` 运行时；禁止在此堆 Rust / host 接线。
