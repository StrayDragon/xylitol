# designing/

跨面 **交互设计稿**（现在只有 `tui/`；以后可加 `web/` 等，禁止为空端预留空目录）。**人类主视图是 shell 页**（`/tui`）：一帧真实产品渲染——由 `just export-design-frame` 从产品代码导出（零手绘）。总览标题是「**场景**」，标题旁单按钮显示当前终端背景（亮色背景/暗色背景），点击切换；标题下方 chips 是不同**场景时刻**（busy/idle）。画布纯展示（鼠标移动无高亮，文本可框选复制）；右栏「区域」条目悬停在画布上高亮对应区域、点击展开该区域的设计稿与分析（模块 intent/draft）。区域清单在右栏，深链 `/tui/<module>` 直达选中区域；帧（busy/idle/short）与终端亮暗同样可深链：`/tui/<module>?view=busy|idle|short&scheme=dark|light`（默认暗色；短终端不出 chips，仅此深链）。tui-lab 候选不进 shell 页，经 `/tui-lab/<id>/<state>` 直达固定态预览。预览是 pathname 不是 hash；右栏 **复制路径** 给出当前 endpoint / module，以及仓库相对路径（不夹 intent 正文）。

**代码是运行时真值。** 改 TUI 先读 [`src/app/tui/`](../src/app/tui/) 产品代码。本稿是对照辅助，不是第二套视觉 SSOT。浏览器格子比 / 差分 / 鼠标 / 流式仍以 host 为准。

Token 色板数据：[`src/app/tui/DESIGN.md`](../src/app/tui/DESIGN.md) frontmatter（一份）。引擎库：`packages/xylitol-tui/AGENTS.md`。产品面：[`src/app/tui/AGENTS.md`](../src/app/tui/AGENTS.md)。

旧 `src/app/tui/design/`（playground 与指针 stub）已整体删除——设计稿入口即本目录 `tui/modules/<id>/`，无中间跳转层。

## 目录

```text
designing/
  AGENTS.md
  app/                 # bun + Vite（无前端框架）；package.json 只在这里
  generated/           # AGENT-INDEX.md、tokens.css/js、shell-frame.json；勿手改
  tui/
    shell.regions.yaml # shell 页区域注解（module + 文本锚点 contains + rows 覆盖行数）
    modules/<id>/      # 产品面（已交付 / 已定案）
      draft.yaml       # 摘要 / 位置与边界（fixed·item·todo-bar）/ todos / 组件快捷键 / 备注
      intent.md        # 可观察 MUST（软顶 ~80 行）
      states/*.yaml    # cell/span 固定态（tui-lab 与闸仍在用；shell 页不直接渲染）
  tui-lab/modules/<id>/  # 实验原型区（未接入产品），内部结构与上面相同
```

不放 `tools/`，不进 `packages/xylitol-tui`。未交付的 Web 端 **不要**先建空 `web/`。

## 帧与区域（shell 页数据流）

1. `just export-design-frame`：`src/app/tui/lab_design_frame.rs`（`#[ignore]` lab）用 harness 起真实会话渲染 busy / idle / short 三帧 → `generated/shell-frame.json`。改了产品 UI 就重跑，**勿手改 JSON**。
2. `tui/shell.regions.yaml`：区域 = `module` + `contains`（行内文本锚点，渲染时定位首条命中行）+ `rows`（覆盖行数）。改布局重导出即可，锚点只在文案变化时跟随；`check_tui_designing` 校验 module 存在与锚点帧齐备。

## 给人 / 给 Agent

| 受众 | 默认读 | 禁止默认读 |
|---|---|---|
| **Agent 改某表面** | 产品代码 → 本目录 `tui/modules/<id>/intent.md` + `draft.yaml`（+ `states/*.yaml`） | `app/` 源码、`generated/*`、整包 HTML、`node_modules` |
| **Agent 找路** | [`generated/AGENT-INDEX.md`](./generated/AGENT-INDEX.md) | 旧 `design/*.md` 正文（已缩成指针） |
| **人类** | `just open-designing` → `/tui`（总览标题「场景」+ 旁侧按钮显示当前终端背景、点击切换；chips = 场景时刻 busy/idle；短终端仅 `?view=short`；悬停/点击右栏区域条目看设计稿；帧与亮暗可经 URL `?view=&scheme=` 直达，默认暗色；tui-lab 经 `/tui-lab/<id>/<state>` 看固定态） | — |

改模块后跑 `just gen-designing-index`；改了产品 UI 重跑 `just export-design-frame`（漏跑由 `scripts/check_tui_designing.py` 进 `just qa` 抓住区域/帧不齐）。

intent.md 只写 **可观察 MUST**（词表、禁止滑入、轨/flush、跨面同源）。禁止钉 Rust 路径 / 类型 / 行数。

## 硬约束

- 色/尺寸只引用 DESIGN token（YAML `token: muted` → `var(--muted)`）。MUST NOT 另立冲突 hex。
- 改 token：`just sync-tui-tokens`，闸 `just check-tui-tokens`。生成物只在 `generated/`；**勿手改**。
- 静图 UI MUST NOT 用「(包)」分层样式；MUST NOT 顶栏快捷键墙。
- **无独立快捷键设计**。组件自己的键写在该模块 `draft.yaml` `keys:`（仅当非空时页面展示）。
- MUST NOT 称 `agent_demo` 为产品 playground。MUST NOT 把 demo 文案回写成产品固定区词表。
- 信息面用词：[`docs/architecture/TUI信息面与固定区词汇.md`](../docs/architecture/TUI信息面与固定区词汇.md)。
- `packages/xylitol-tui` **不**平行维护 design HTML。
- shell 帧是产品渲染导出：MUST NOT 在 `shell-frame.json` / regions 锚点里手改视觉内容来「修设计」——要改设计先改产品或先立 change。

## tui-lab 实验区（交互候选）

**UI/UX 迭代 SOP：新交互 / 新改动先进本区 staged（实验原型），评审定案后按「晋级」落产品面；不做即「淘汰」删目录。** 后续 UI 演进以此 SOP 驱动，不直接在产品面试错。

产品面模块树（`tui/modules/`）之外的第二棵树：`designing/tui-lab/modules/<id>/`，预览 endpoint
`/tui-lab/<id>/<state>`。**位置即语义**——tui-lab 内模块一律是未定案的交互候选 / 原型，
MUST NOT 再在正文里散布「未交付」「候选」字样。

| 规则 | 内容 |
|---|---|
| 写什么 | 新交互想法、原型固定态、待人类评审的取舍点（放 `draft.yaml` `todos:` / `notes:`） |
| draft.yaml | `surface: tui-lab`（晋级时改回目标面）；其余字段与产品模块同构 |
| 晋级 | `git mv` 到 `tui/modules/<id>/` + `surface: tui` + 补齐 harness/BDD 护栏 + 跑 `just gen-designing-index` |
| 淘汰 | 整目录删除，无 tombstone |
| 边界 | lab 模块不得被产品代码引用；产品固定区词表不受 lab 文案影响 |
