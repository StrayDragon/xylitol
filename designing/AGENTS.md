# designing/

跨面 **交互设计稿**（现在只有 `tui/`；以后可加 `web/` 等，禁止为空端预留空目录）。人类用浏览器看固定态、对齐、待办和备注；Agent 只读短模块。预览地址是 pathname **`/tui/<id>/<state>`**（以后可加 `/web/…`），不是 hash。右栏 **复制路径** 给出当前 endpoint / module / state / last_action，以及仓库相对路径（不夹 intent 正文）。

**代码是运行时真值。** 改 TUI 先读 [`src/app/tui/`](../src/app/tui/) 产品代码。本稿是对照辅助，不是第二套视觉 SSOT。浏览器格子比 / 差分 / 鼠标 / 流式仍以 host 为准。

Token 色板数据：[`src/app/tui/DESIGN.md`](../src/app/tui/DESIGN.md) frontmatter（一份）。引擎库：`packages/xylitol-tui/AGENTS.md`。产品面：[`src/app/tui/AGENTS.md`](../src/app/tui/AGENTS.md)。

旧 `src/app/tui/design/playground/` 已拆除。`src/app/tui/design/*.md` 只留指针。

## 目录

```text
designing/
  AGENTS.md
  app/                 # bun + Vite；package.json 只在这里
  generated/           # AGENT-INDEX.md、tokens.css/js；勿手改
  tui/modules/<id>/    # 产品面（已交付 / 已拍板）
    draft.yaml         # 摘要 / 对齐（chrome·item·todo-bar）/ todos / 组件键 / 备注
    intent.md          # 可观察 MUST（软顶 ~80 行）
    states/*.yaml      # cell/span 固定态
  lab/modules/<id>/    # 实验原型区（未接入产品），内部结构与上面相同
```

不放 `tools/`，不进 `packages/xylitol-tui`。未交付的 Web 端 **不要**先建空 `web/`。

## 给人 / 给 Agent

| 受众 | 默认读 | 禁止默认读 |
|---|---|---|
| **Agent 改某表面** | 产品代码 → 本目录 `tui/modules/<id>/intent.md` + `draft.yaml` + `states/*.yaml` | `app/` 源码、`generated/*.css`、整包 HTML、`node_modules` |
| **Agent 找路** | [`generated/AGENT-INDEX.md`](./generated/AGENT-INDEX.md) | 旧 `design/*.md` 正文（已缩成指针） |
| **人类** | `just open-designing` → `/tui/<id>/<state>`（上方按钮切态；动画只用播放按钮；右栏 marked + 复制路径） | — |

改模块后跑 `just gen-designing-index`（漏跑由 `scripts/check_tui_designing.py` 进 `just qa` 抓住）。

intent.md 只写 **可观察 MUST**（词表、禁止滑入、轨/flush、跨面同源）。禁止钉 Rust 路径 / 类型 / 行数。

## 硬约束

- 色/尺寸只引用 DESIGN token（YAML `token: muted` → `var(--muted)`）。MUST NOT 另立冲突 hex。
- 改 token：`just sync-tui-tokens`，闸 `just check-tui-tokens`。生成物只在 `generated/`；**勿手改**。
- 静图 UI MUST NOT 用「(包)」分层样式；MUST NOT 顶栏快捷键墙。
- **无独立快捷键设计**。组件自己的键写在该模块 `draft.yaml` `keys:`（仅当非空时页面展示）。
- MUST NOT 称 `agent_demo` 为产品 playground。MUST NOT 把 demo 文案回写成产品 chrome 词表。
- 信息面用词：[`docs/architecture/TUI信息面与chrome词汇.md`](../docs/architecture/TUI信息面与chrome词汇.md)。
- `packages/xylitol-tui` **不**平行维护 design HTML。

## tui-lab 实验区（交互候选）

产品面模块树（`tui/modules/`）之外的第二棵树：`designing/tui-lab/modules/<id>/`，预览 endpoint
`/tui-lab/<id>/<state>`。**位置即语义**——tui-lab 内模块一律是未拍板的交互候选 / 原型，
MUST NOT 再在正文里散布「未交付」「候选」字样。

| 规则 | 内容 |
|---|---|
| 写什么 | 新交互想法、原型固定态、待人类评审的取舍点（放 `draft.yaml` `todos:` / `notes:`） |
| draft.yaml | `surface: tui-lab`（晋级时改回目标面）；其余字段与产品模块同构 |
| 晋级 | `git mv` 到 `tui/modules/<id>/` + `surface: tui` + 补齐 harness/BDD 护栏 + 跑 `just gen-designing-index` |
| 淘汰 | 整目录删除，无 tombstone |
| 边界 | lab 模块不得被产品代码引用；产品 chrome 词表不受 lab 文案影响 |
