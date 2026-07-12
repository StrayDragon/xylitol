# src/app/tui/design/

本目录 = 产品 TUI **唯一视觉 SSOT**（MUST）+ 人类用浏览器 playground。

Token / 全局 Overview：上一级 [`../DESIGN.md`](../DESIGN.md)。包引擎边界：`packages/xylitol-tui/AGENTS.md`。本面接线：[`../AGENTS.md`](../AGENTS.md)。

## 一份 DESIGN · 两个预览面

| 面 | 路径 | 干什么 |
|---|---|---|
| **MUST（SSOT）** | 本目录 `*.md` + [`../DESIGN.md`](../DESIGN.md) | 色板、壳、键位、呈现规则 |
| **活实验场** | `packages/xylitol-tui/examples/agent_demo.rs`（`just demo-tui`） | **产品 TUI 的快速 playground**：组件形状 / 交互先在此试，再进 host |
| **浏览器静图** | [`playground/`](./playground/) | 审 token / 整壳合成；非运行时 |

包**不**另维护平行 design HTML。`Palette` = DESIGN 的运行时快照。分发库后仍以本路径为文档引用（或嵌入方自备 token）。

## 给人 / 给 agent

| 受众 | 读什么 |
|---|---|
| **人类** | `*.md` MUST；实验用 `just demo-tui`；审色开 [`playground/`](./playground/) |
| **Agent** | 默认读 `DESIGN.md` + 相关 `design/<comp>.md`。**默认忽略** `playground/` HTML |

Agent **仅在**人类指定路径或粘贴 playground 片段时读 HTML。不要把 playground 当实现真值；落地以 MUST + `agent_demo` / 包测试 + 产品 host 为准。

## 硬约束

- 子文档 MUST 声明 `tokens_from: "../DESIGN.md"`；MUST NOT 另立冲突 hex。
- playground 色值 MUST 来自 `DESIGN.md` frontmatter。改 token 后跑：
  `python3 src/app/tui/design/playground/sync_tokens.py`，并对齐 `packages/xylitol-tui` 的 `Palette`。
- playground 是产品可交互设计图，**不是**运行时；禁止在此堆 Rust / host 接线。
- 窄宽 / 空态 / 焦点细行为以 `just demo-tui` + 包 harness 为准；本 playground 对 Widgets/Atoms/Ask 仅弱对照。
