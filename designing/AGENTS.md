# designing/

跨端 **交互设计稿**（产品端稿 `tui/`；**禁止**为空端预留空目录。实验原型区 `tui-lab/` 见下节）。人类主视图：`just open-designing` → `/tui`（产品代码导出的真实帧 + 右栏区域）。tui-lab 经 `/tui-lab/<id>/<state>`，不进 shell 页。深链与亮暗/帧查询见 app；预览是 pathname 不是 hash。右栏 **复制路径**（copy-handoff）给出当前 endpoint / surface / module / state 等元信息与相关设计文档的仓库相对路径清单，便于 handoff 定位。

**代码是运行时真值。** 改 TUI 先读 [`src/app/tui/`](../src/app/tui/)。本稿是对照辅助，不是第二套视觉 SSOT。浏览器格子比 / 差分 / 鼠标 / 流式仍以 host 为准。

Token 色板：[`src/app/tui/DESIGN.md`](../src/app/tui/DESIGN.md) frontmatter（一份）。引擎：`packages/xylitol-tui/AGENTS.md`。产品端：[`src/app/tui/AGENTS.md`](../src/app/tui/AGENTS.md)。设计稿入口即本目录 `tui/modules/<id>/`。

## 目录

```text
designing/
  AGENTS.md
  app/                 # bun + Vite（无前端框架）；package.json 只在这里
  generated/           # AGENT-INDEX.md、tokens.css/js、shell-frame.json；勿手改
  tui/
    shell.regions.yaml # shell 页区域注解（module + 文本锚点 contains + rows 覆盖行数）
    modules/<id>/      # 产品端（已交付 / 已定案）
      draft.yaml       # 摘要 / 位置与边界（fixed·item·todo-bar）/ todos / 组件快捷键 / 备注
      intent.md        # 可观察 MUST（软顶 ~80 行）
      states/*.yaml    # cell/span 固定态（tui-lab 与门禁仍在用；shell 页不直接渲染）
  tui-lab/modules/<id>/  # 实验原型区（未接入产品），内部结构与上面相同
```

不放 `tools/`，不进 `packages/xylitol-tui`。未交付的端 **不要**先建空目录。

## 帧与区域（shell 页数据流）

1. `just export-design-frame`：`src/app/tui/lab_design_frame.rs`（`#[ignore]` lab）用 harness 起真实会话渲染 busy / idle / short 三帧 → `generated/shell-frame.json`。改了产品 UI 就重跑，**勿手改 JSON**。
2. `tui/shell.regions.yaml`：区域 = `module` + `contains`（行内文本锚点，渲染时定位首条命中行）+ `rows`（覆盖行数）。改布局重导出即可，锚点只在文案变化时跟随；`check_tui_designing` 校验 module 存在与锚点帧齐备。

## 给人 / 给 Agent

| 受众 | 默认读 | 禁止默认读 |
|---|---|---|
| **Agent 改某端** | 产品代码 → 本目录 `tui/modules/<id>/intent.md` + `draft.yaml`（+ `states/*.yaml`） | `app/` 源码、`generated/*`、整包 HTML、`node_modules` |
| **Agent 找路** | [`generated/AGENT-INDEX.md`](./generated/AGENT-INDEX.md) | `app/` 源码、整包 HTML |
| **人类** | `just open-designing` → `/tui`；tui-lab 经 `/tui-lab/<id>/<state>` | — |

改模块后跑 `just gen-designing-index`；改了产品 UI 重跑 `just export-design-frame`（漏跑由 `scripts/check_tui_designing.py` 进 `just qa` 抓住区域/帧不齐）。

intent.md 只写 **可观察 MUST**（词表、禁止滑入、轨/flush、跨端同源）。禁止钉 Rust 路径 / 类型 / 行数。

## 硬约束

| 规则 | 禁止 |
|---|---|
| 颜色 / 尺寸只引用 DESIGN token（YAML `token: muted` → `var(--muted)`） | 另立冲突 hex |
| 改 token：`just sync-tui-tokens`；门禁 `just check-tui-tokens`；生成物只在 `generated/` | 手改生成物 |
| 静图 UI 无「(包)」分层样式；无顶栏快捷键墙 | 把包分层样式当产品表达 |
| **无独立快捷键设计**：组件自己的键写在该模块 `draft.yaml` `keys:`（仅当非空时页面展示） | 另立快捷键设计 / 在散文里钉键位 |
| `agent_demo` 不是产品 playground / 固定区真值（硬边界见 `packages/xylitol-tui/AGENTS.md`） | 拿 demo 当产品定稿 |
| 信息呈现用词：[`docs/architecture/TUI信息呈现与固定区词汇.md`](../docs/architecture/TUI信息呈现与固定区词汇.md) | 发明第二套同义词 |
| `packages/xylitol-tui` **不**平行维护 design HTML | 在包里另起 design 文档树 |
| shell 帧是产品渲染导出；要改设计先改产品或先立 change | 在 `shell-frame.json` / regions 锚点里手改视觉内容来「修设计」 |

## tui-lab 实验区（交互候选）

**UI/UX 迭代 SOP：新交互 / 新改动先进本区 staged（实验原型），评审定案后按「晋级」落产品端；不做即「淘汰」删目录。** 后续 UI 演进以此 SOP 驱动，不直接在产品端试错。

产品端模块树（`tui/modules/`）之外的第二棵树：`designing/tui-lab/modules/<id>/`，预览 endpoint
`/tui-lab/<id>/<state>`。**位置即语义**——tui-lab 内模块一律是未定案的交互候选 / 原型，
MUST NOT 再在正文里散布「未交付」「候选」字样。

| 规则 | 内容 |
|---|---|
| 写什么 | 新交互想法、原型固定态、待人类评审的取舍点（放 `draft.yaml` `todos:` / `notes:`） |
| draft.yaml | `surface: tui-lab`（晋级时改回目标端）；其余字段与产品模块同构 |
| 晋级 | `git mv` 到 `tui/modules/<id>/` + `surface: tui` + 补齐 harness/BDD 护栏 + 跑 `just gen-designing-index` |
| 淘汰 | 整目录删除，无 tombstone |
| 边界 | lab 模块不得被产品代码引用；产品固定区词表不受 lab 文案影响 |
