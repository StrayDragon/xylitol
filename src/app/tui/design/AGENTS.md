# src/app/tui/design/

本目录 = 产品 TUI（`src/app/tui`）**唯一视觉 SSOT**（MUST）+ 浏览器**静态设计图**。

Token / 全局 Overview：上一级 [`../DESIGN.md`](../DESIGN.md)。引擎库：`packages/xylitol-tui/AGENTS.md`。产品面：[`../AGENTS.md`](../AGENTS.md)。

## 预览职责（勿混）

| 层 | 路径 | 干什么 | 不干什么 |
|---|---|---|---|
| **DESIGN playground** | [`playground/index.html`](./playground/index.html) | **产品面专用**静图：固定状态的形状 / 色板 / 整壳对照（**唯一** design playground SSOT）；**UiEntry 默认 rail** | **不是** `agent_demo`；不教快捷键百科；不当实现真值 |
| **生产实现** | `src/app/tui/` | XyDriver / bridge / layout 真接线 | 不在本目录再实现通用引擎组件 |
| **包交互演示**（可选） | `just demo-tui`（`packages/…/agent_demo`） | 引擎 / 通用组件可交互试跑 | **允许与产品 chrome / 文案有差异**；**不**充当本目录 playground；不替代产品 host |

**UiEntry 默认**：tool/bash/diff = status 左边轨 + gutter；user / assistant / thinking = flush。合约：`app-tui-transcript` / `app-tui-chrome`（c1830）。

**MUST（合约文）**：本目录 `*.md` + [`../DESIGN.md`](../DESIGN.md)。playground HTML 文案 **不是**合约。信息面用词：[`docs/architecture/TUI信息面与chrome词汇.md`](../../../../docs/architecture/TUI信息面与chrome词汇.md)。

包**不**另维护平行 design HTML。`Palette` = DESIGN 的运行时快照。

## 给人 / 给 agent

| 受众 | 读什么 |
|---|---|
| **人类** | `*.md` MUST；静图开 `just open-design-playground`；真产品行为以 `src/app/tui` / `cargo run` 为准 |
| **Agent** | 默认读 `DESIGN.md` + 相关 `design/<comp>.md`。**默认忽略** `playground/` HTML |

Agent **仅在**人类指定路径或粘贴 playground 片段时读 HTML。落地以 MUST + 产品 host / 产品测为准；`agent_demo` 仅在改包引擎 / 通用组件时相关。

## 硬约束

- 子文档 MUST 声明 `tokens_from: "../DESIGN.md"`；MUST NOT 另立冲突 hex。
- playground 色值 MUST 来自 `DESIGN.md` frontmatter。改 token 后跑：
  `just sync-tui-tokens`，并对齐 `Palette`；闸门：`just check-tui-tokens`（已进 `just qa`）。
  打开静图：`just open-design-playground`。
- playground SSOT = **产品**静态确定状态的设计图（仅 `index.html`）；**不是**运行时，禁止堆 Rust / host 接线。
- 窄宽 / 空态 / 焦点细行为以 **产品 host** + 产品 / 包 harness 为准（包 demo 可对照，但非本 SSOT）。
- 静图 UI **MUST NOT** 用「(包)」、置灰、次级样式等表达实现分层；槽一律平等。
- 静图 UI **MUST NOT** 在顶栏堆快捷键提示墙。
- 改 playground / `design/*.md` 后 MUST 跑：
  `python3 scripts/check_tui_design_playground.py --check`（经 `just check-scripts` / `just qa`）；
  色板另跑 `just check-tui-tokens`。夹具：`design/fixtures/*.yaml`。
  **禁止**只靠人眼发现 DESIGN 漂移。
