# src/app/tui/design/

> **指针**：交互设计稿在仓库顶层 [`designing/`](../../../../designing/)（`just open-designing`）。本目录 `*.md` 只留迁移指针，不再维护正文。

Token 色板数据仍在 [`../DESIGN.md`](../DESIGN.md) frontmatter。运行时真值 = [`../`](../) 产品代码。

引擎库：`packages/xylitol-tui/AGENTS.md`。产品面：[`../AGENTS.md`](../AGENTS.md)。设计稿操作边界：[`designing/AGENTS.md`](../../../../designing/AGENTS.md)。

## 预览职责（勿混）

| 层 | 路径 | 干什么 | 不干什么 |
|---|---|---|---|
| **产品代码** | `src/app/tui/` | XyDriver / layout / paint | 不在浏览器里再实现一遍 |
| **designing** | [`designing/`](../../../../designing/) | 交互设计稿：固定态 + 对齐 + 待办 + 备注 | 不是产品真值；不是 wasm TUI；不是 `agent_demo` |
| **本目录** | `design/*.md` | 旧路径指针 | 不当第二套 SSOT |
| **包交互演示** | `just demo-tui` | 引擎 / 通用组件可交互试跑 | 允许与产品 chrome / 文案有差异 |

### Agent 防误导（硬）

- **MUST NOT** 称 `agent_demo` 为「动态 playground / 产品 live playground」。
- **MUST NOT** 把 demo seed / footer / plate 文案回写成产品中文 chrome 词表。
- **MUST NOT** 以 demo 行为否定 designing `intent.md` MUST；冲突时以**产品代码**为准，designing 为辅助。
- 信息面用词（产品）：[`docs/architecture/TUI信息面与chrome词汇.md`](../../../../docs/architecture/TUI信息面与chrome词汇.md)。
- **无独立快捷键设计**；组件键在各模块 `draft.yaml` `keys:`。

**UiEntry 默认**：tool/bash/diff = status 左边轨 + gutter；user / assistant / thinking = flush。

包**不**另维护平行 design HTML。`Palette` = DESIGN 的运行时快照。

## 给人 / 给 agent

| 受众 | 读什么 |
|---|---|
| **人类** | `just open-designing`；真产品行为以 `src/app/tui` / `cargo run` 为准 |
| **Agent** | 产品代码 → [`designing/AGENTS.md`](../../../../designing/AGENTS.md) + `generated/AGENT-INDEX.md` + 相关模块。**默认忽略** `designing/app/` |

## 硬约束

- 色值 MUST 来自 `DESIGN.md` frontmatter。改 token：`just sync-tui-tokens`；闸：`just check-tui-tokens`。
- 静图 UI **MUST NOT** 用「(包)」表达实现分层；**MUST NOT** 顶栏快捷键墙。
- 改 designing 后 MUST 跑 `python3 scripts/check_tui_designing.py --check`（经 `just qa`）与 `just check-tui-tokens`。**禁止**只靠人眼发现 DESIGN 漂移。
