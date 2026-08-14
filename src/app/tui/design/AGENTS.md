# src/app/tui/design/

> **superseded（意图 / 静图）**：人与 Agent 的默认入口改为 [`../designing/AGENTS.md`](../designing/AGENTS.md)。本目录 `*.md` + `playground/index.html` 是 **双轨旧源**，拆除条件见 c2230。Token 色板数据仍在 [`../DESIGN.md`](../DESIGN.md) frontmatter。

Token / 全局 Overview：上一级 [`../DESIGN.md`](../DESIGN.md)。引擎库：`packages/xylitol-tui/AGENTS.md`。产品面：[`../AGENTS.md`](../AGENTS.md)。

## 预览职责（勿混）

| 层 | 路径 | 干什么 | 不干什么 |
|---|---|---|---|
| **designing** | [`../designing/`](../designing/) | **意图 + 结构化静图**（模块目录）；人类浏览器；Agent 默认读这里 | 不是产品运行时；不是 `agent_demo` |
| **DESIGN playground**（双轨） | [`playground/index.html`](./playground/index.html) | 未迁槽的旧静图；闸对其它槽仍解析 HTML | **不是** 唯一 SSOT；不当实现真值 |
| **生产实现** | `src/app/tui/` | XyDriver / bridge / layout 真接线 | 不在本目录再实现通用引擎组件 |
| **包交互演示**（可选） | `just demo-tui`（`packages/…/agent_demo`） | 引擎 / 通用组件可交互试跑 | **允许与产品 chrome / 文案有差异**；不充当 designing |

### Agent 防误导（硬）

- **MUST NOT** 称 `agent_demo` 为「动态 playground / 产品 live playground」。
- **MUST NOT** 把 demo seed / footer / plate 文案回写成产品中文 chrome 词表（队列条、滚动提示、命令面板…）——那是**产品面**约束；包 demo 用自己的英文/plate 用语即可。
- **MUST NOT** 以 demo 行为否定 designing `intent.md` MUST；冲突时以 designing + 产品 host 为准。
- 信息面用词（产品）：[`docs/architecture/TUI信息面与chrome词汇.md`](../../../../docs/architecture/TUI信息面与chrome词汇.md)。

**UiEntry 默认**：tool/bash/diff = status 左边轨 + gutter；user / assistant / thinking = flush。合约：`app-tui-transcript` / `app-tui-chrome`（c1830）。

**MUST（合约文）**：优先 [`../designing/modules/`](../designing/modules/) 的 `intent.md`。未迁组件仍可读本目录 `*.md`。playground HTML 文案 **不是**合约。

包**不**另维护平行 design HTML。`Palette` = DESIGN 的运行时快照。

## 给人 / 给 agent

| 受众 | 读什么 |
|---|---|
| **人类** | `just open-designing`；未迁槽可暂开 `just open-design-playground`；真产品行为以 `src/app/tui` / `cargo run` 为准 |
| **Agent** | 默认 [`../designing/AGENTS.md`](../designing/AGENTS.md) + `generated/AGENT-INDEX.md` + 相关模块。**默认忽略** `playground/` HTML 与 `designing/app/` |

Agent **仅在**人类指定路径或粘贴 playground 片段时读 HTML。落地以 MUST + 产品 host / 产品测为准；`agent_demo` 仅在改包引擎 / 通用组件时相关。

## 硬约束

- 子文档 MUST 声明 `tokens_from: "../DESIGN.md"`；MUST NOT 另立冲突 hex。
- 色值 MUST 来自 `DESIGN.md` frontmatter。改 token 后跑：
  `just sync-tui-tokens`，并对齐 `Palette`；闸门：`just check-tui-tokens`（已进 `just qa`）。
  打开意图静图：`just open-designing`。旧文件：`just open-design-playground`。
- playground 双轨期仍禁止堆 Rust / host 接线。
- 窄宽 / 空态 / 焦点细行为以 **产品 host** + 产品 / 包 harness 为准（包 demo 可对照，但非本 SSOT）。
- 静图 UI **MUST NOT** 用「(包)」、置灰、次级样式等表达实现分层；槽一律平等。
- 静图 UI **MUST NOT** 在顶栏堆快捷键提示墙。
- 改 playground / `design/*.md` / `designing/` 后 MUST 跑：
  `python3 scripts/check_tui_design_playground.py --check`（经 `just check-scripts` / `just qa`）；
  `python3 scripts/check_tui_designing.py --check`；
  色板另跑 `just check-tui-tokens`。已迁模块夹具走 `designing/modules/*/states`。
  **禁止**只靠人眼发现 DESIGN 漂移。
