# src/app/tui/designing/

产品 TUI **视觉意图 + 结构化静图** 目录。人类用浏览器预览固定态；Agent 读短模块，不吞整站 HTML。

Token 色板数据仍在 [`../DESIGN.md`](../DESIGN.md) frontmatter（一份）。运行时真值 = [`../`](../) 产品代码。本目录 **不是** 产品真值（格子比 / 差分 / 鼠标 / 流式仍以 host 为准）。

本目录 **supersede** [`../design/AGENTS.md`](../design/AGENTS.md) 里「playground 唯一静图 SSOT / Agent 忽略 HTML」的过时句。旧 `design/*.md` + `design/playground/index.html` 双轨至拆除日（见 c2230 提案）。

引擎库：`packages/xylitol-tui/AGENTS.md`。产品面：[`../AGENTS.md`](../AGENTS.md)。

## 两源（硬）

| 真值 | 路径 | 干什么 | 不干什么 |
|---|---|---|---|
| **产品代码** | `src/app/tui/` | XyDriver / layout / paint | 不在浏览器里再实现一遍 |
| **designing** | 本目录 | 意图 MUST + YAML 固定态 + token 预览 | 不是 wasm TUI、不是 HostSession、不是 `agent_demo` |

浏览器预览 = 意图对照（layoutlib）。真机 = 产品 host。

## 给人 / 给 Agent

| 受众 | 默认读 | 禁止默认读 |
|---|---|---|
| **Agent 改某表面** | `modules/<id>/intent.md`（软顶 ~80 行）+ `states/*.yaml` | `app/` 源码、`generated/*.css`、整包 HTML、`node_modules`、旧 `playground/index.html` |
| **Agent 找路** | [`generated/AGENT-INDEX.md`](./generated/AGENT-INDEX.md) | 29 个旧 `design/*.md` 一次性灌入 |
| **人类** | `just open-designing` + 偶尔 intent | — |

改模块后跑 `just gen-designing-index`（漏跑由 `scripts/check_tui_designing.py` 进 `just qa` 抓住）。

intent.md 只写 **可观察 MUST**（词表、禁止滑入、轨/flush、跨面同源）。禁止钉 Rust 路径 / 类型 / 行数。

## 硬约束

- 色/尺寸只引用 DESIGN token（YAML `token: muted` → `var(--muted)`）。MUST NOT 另立冲突 hex。
- 改 token：`just sync-tui-tokens`，闸 `just check-tui-tokens`。生成物在 `generated/`（以及双轨期的 playground）；**勿手改**。
- 静图 UI MUST NOT 用「(包)」分层样式；MUST NOT 顶栏快捷键墙。
- MUST NOT 称 `agent_demo` 为产品 playground。MUST NOT 把 demo 文案回写成产品 chrome 词表。
- 信息面用词：[`docs/architecture/TUI信息面与chrome词汇.md`](../../../../docs/architecture/TUI信息面与chrome词汇.md)。
- `packages/xylitol-tui` **不**平行维护 design HTML。
