# src/app/tui/design/

本目录 = 产品 TUI **唯一视觉 SSOT**（MUST）+ 浏览器**静态设计图**。

Token / 全局 Overview：上一级 [`../DESIGN.md`](../DESIGN.md)。引擎库：`packages/xylitol-tui/AGENTS.md`。产品面：[`../AGENTS.md`](../AGENTS.md)。

## 三层预览职责（勿混）

| 层 | 路径 | 干什么 | 不干什么 |
|---|---|---|---|
| **静态设计图** | [`playground/`](./playground/) | 固定状态的形状 / 色板 / 整壳对照 | 不表达实现分层（无置灰/「包」标签）；不教快捷键百科；不当实现真值 |
| **动态 playground** | `just demo-tui`（`agent_demo`） | 可交互试形状与键位 | 不替代产品 host |
| **生产实现** | `src/app/tui/` | XyDriver / bridge / layout 真接线 | 不在本目录再实现通用引擎组件 |

**MUST（合约文）**：本目录 `*.md` + [`../DESIGN.md`](../DESIGN.md)。

包**不**另维护平行 design HTML。`Palette` = DESIGN 的运行时快照。

## 给人 / 给 agent

| 受众 | 读什么 |
|---|---|
| **人类** | `*.md` MUST；静态图开 playground；交互用 `just demo-tui` |
| **Agent** | 默认读 `DESIGN.md` + 相关 `design/<comp>.md`。**默认忽略** `playground/` HTML |

Agent **仅在**人类指定路径或粘贴 playground 片段时读 HTML。落地以 MUST + `agent_demo` / 包测试 + 产品 host 为准。

## 硬约束

- 子文档 MUST 声明 `tokens_from: "../DESIGN.md"`；MUST NOT 另立冲突 hex。
- playground 色值 MUST 来自 `DESIGN.md` frontmatter。改 token 后跑：
  `just sync-tui-tokens`，并对齐 `Palette`；闸门：`just check-tui-tokens`（已进 `just qa`）。
  打开静图：`just open-design-playground`。
- playground = **静态确定状态**的设计图（可切换示意态），**不是**运行时，禁止堆 Rust / host 接线。
- 窄宽 / 空态 / 焦点细行为以 `just demo-tui` + 包 harness 为准。
- 静图 UI **MUST NOT** 用「(包)」、置灰、次级样式等表达实现分层；槽一律平等。
- 静图 UI **MUST NOT** 在顶栏堆快捷键提示墙。
- 改 playground / `design/*.md` 后 MUST 跑：
  `python3 scripts/check_tui_design_playground.py --check`（经 `just check-scripts` / `just qa`）；
  色板另跑 `just check-tui-tokens`。夹具：`design/fixtures/*.yaml`。
  **禁止**只靠人眼发现 DESIGN 漂移。
