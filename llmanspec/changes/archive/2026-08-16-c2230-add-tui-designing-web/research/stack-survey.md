# designing 栈调研（c2230）

> 现网文档（2026-08-14）。每类 ≥3 候选 + 1 推荐 + 否决理由。本票 **不**实现 HostSession / PreviewInject。

## A. 设计应用壳（给人浏览模块）

要：bun 一等；启动快；适合 **文档 + 状态目录**，不是 React 组件车间。本应用的「组件」是 TUI 表面（activity-fold、toast…），不是 Button/Dialog。原则：**壳可以丑，模块目录必须清。**

| 候选 | 证据 | 适合？ | 否决 / 采用 |
|---|---|---|---|
| **自研 bun + Vite + 少量 DOM** | bun 官方：`bun create vite`、`bun --bun vite dev`（[bun.sh/docs/guides/ecosystem/vite](https://bun.sh/docs/guides/ecosystem/vite)）。Vite 8 认 `bun.lock`（[vitejs/vite optimizer](https://github.com/vitejs/vite)）。vanilla TS，零 CSF。 | **是**：YAML states 当数据；壳只做导航 + 预览窗。 | **推荐** |
| **Ladle** | 官方：「drop-in alternative to Storybook for **React** components」；故事 = `*.stories.tsx` + `@ladle/react`（[ladle.dev/docs](https://ladle.dev/docs)、[ladle.dev/docs/stories](https://ladle.dev/docs/stories)）。 | 否：锁进 React CSF；不直接吃 YAML states。 | 否决。除非我们先把 TUI 表面写成 React 组件——与本票相反。 |
| **Storybook** | 生态最大（addons / Chromatic / CSF）。对「终端静图目录」过重。 | 否：不可替代点不存在（无视觉回归 SaaS 需求）。 | **默认否决** |
| **Histoire** | Vite playground；CI 矩阵是 Vue 3 / Svelte / Nuxt / SvelteKit（[histoire-dev/histoire](https://github.com/histoire-dev/histoire)）。本仓无 Vue/Svelte。 | 否 | 否决 |
| **VitePress / fumadocs / Starlight** | 文档站。预览槽要在 MDX 里嵌自定义组件，目录心智是「文章」不是「固定态」。 | 弱 | 否决作壳。intent.md 已是文档；不需要第二套文档框架。 |
| **shadcn / Base UI / Park UI** | 产品级控件库。 | **只**当壳按钮/列表，不当 TUI 像素。本票壳用原生 `<button>` + token CSS 即可。 | 不引入。壳丑是特性。 |

**推荐（A）**：`src/app/tui/designing/app/` 内 bun + Vite vanilla TS。不引入 React/Vue。模块列表 = 扫 `modules/*/preview.ts`。

## B. 终端「看起来像 TTY」的渲染

要：固定态可预览；人能感到格子/轨/折叠；**结构化数据可 round-trip**（Agent 读 states，不必 OCR）。

| 候选 | 证据 | 作者格式？ | 预览器？ | 结论 |
|---|---|---|---|---|
| **结构化 cell-grid**（YAML → DOM 等宽格子，1 cell） | 自研。`must_contain` 直接扫 `lines[].text`。色走 `token: muted` → `var(--muted)`。 | **是** | **是（默认可检）** | **推荐** |
| **xterm.js** `@xterm/xterm` + `@xterm/headless` | 官方 canvas/WebGL 渲染；headless **排除 DOM**、实验性（[xtermjs.org](https://xtermjs.org/)、[npm @xterm/headless](https://www.npmjs.com/package/@xterm/headless)）。选中/搜索/Agent 读 DOM 不友好。 | 若喂 ANSI → 诱人把 ANSI 当设计源 | 最多只读 view | 本票不加。证据推翻点：某 MUST 必须真 SGR 才加 **只读** view。 |
| **Vercel wterm** | DOM 终端；Zig WASM ~12KB；可选 libghostty ~400KB（[vercel-labs/wterm](https://github.com/vercel-labs/wterm/)，v0.2.1 2026-04）。适合 PTY/实时，不适合「YAML 固定态」。 | 否（吃字节流） | 能，但过重 | 否决作作者格式；静图不值得 WASM VT。 |
| **ghostty-web** / **@termless** | 真 VT；ghostty-web ~400KB WASM、xterm API 兼容（[coder/ghostty-web](https://github.com/coder/ghostty-web)）。 | 否 | 过重 | 否决 |
| **asciinema player** | 录像回放。 | 否 | 否 | 否决：录像不是设计模块。 |
| **纯 CSS 仿终端窗口** | 现 playground `.term` 已是窗框。 | 否（内容若手画 HTML = 第二套 layout） | 壳可以 | **窗框用 CSS**；**内容**走 cell-grid，禁止手画第二套 layout。 |

### B 必须回答的题

| # | 答 |
|---|---|
| 1. 设计源数据？ | **cell/span 树**（YAML `lines: [{ text, token }]`）。不是 ANSI，不是 HTML 模板。 |
| 2. 仿真器是预览器还是作者格式？ | **源数据 ≠ xterm/wterm**。仿真器最多当一种只读 view（本切片不加）。 |
| 3. 80×24 vs 窄宽？ | state yaml 可选 `cols:` / `model: narrow\|wide`。沿用已有 `fixtures/models.narrow.yaml` 模式。未写则预览默认 80。 |

**预置倾向未被推翻**：作者格式 = YAML 状态 + 短 intent；预览 = cell-grid DOM；TTY 窗框 CSS。

## C. Token / 结构化尺寸颜色

已有：`DESIGN.md` YAML frontmatter → `sync_tokens.py` → `tokens.css`/`tokens.js` + Rust `Palette`。闸：`just check-tui-tokens`。

| 候选 | 证据 | 结论 |
|---|---|---|
| **保留 frontmatter，只改生成器输出到 designing** | 现闸已禁 playground 手写 hex。生成器已解析 `colors` / `colors_light`。 | **推荐**。零新工具；Palette 对齐路径不拆。 |
| **W3C Design Tokens JSON + Style Dictionary** | DTCG Format Module **2025.10** 已 stable（[w3.org CG-FINAL-format-20251028](https://www.w3.org/community/reports/design-tokens/CG-FINAL-format-20251028/)）。Style Dictionary v4 支持 DTCG，**2025.10 未完全跟上**（[styledictionary.com/info/dtcg](https://styledictionary.com/info/dtcg/)）。 | 否决本票。多一端、多一种 JSON，而消费者只有 CSS + 一份 Rust Palette。无 Figma/Tokens Studio 需求。 |
| **迁到 designing/tokens/*.json 废 frontmatter** | Pre-0.0.1 允许一次性改名，但 Palette 解析与 DESIGN 正文 Overview 仍要接。 | 推迟。等 designing 成意图 SSOT 后再考虑把 frontmatter **生成自** 同一份数据——本票不双写。 |

硬约束兑现：

- **一份**颜色/spacing 数据 = `DESIGN.md` frontmatter（本票不另立）。
- 模块只引用 token 名（`token: muted` → `{colors.muted}` / `var(--muted)`），禁止冲突 hex。
- `just check-tui-tokens` 仍绿；生成物写 playground（双轨）+ `designing/generated/`。

## D. Agent 上下文

见落地 `src/app/tui/designing/AGENTS.md`。摘要：

| 受众 | 默认读 | 禁止默认读 |
|---|---|---|
| Agent 改某表面 | `modules/<id>/intent.md`（软顶 ~80 行）+ `states/*.yaml` | `app/` 源码、生成 CSS、整包 HTML、`node_modules`、旧 2.6k 行 playground |
| Agent 找路 | `generated/AGENT-INDEX.md`（每模块一行） | 29 个旧 `design/*.md` 一次性灌入 |
| 人类 | 浏览器 designing + 偶尔 intent | — |

生成：`just gen-designing-index`。qa 经 `scripts/check_tui_designing.py` 抓过期。

## 与 c2200 的正交（只读调研，不实现 C）

| 文 | 对本票的含义 |
|---|---|
| `code-as-design-and-tui-verify.md` §3 | 不要一次废光文档；要废的是「第二套手写实现」。C 方案（Scene API / HostSession 目录）是 **另一票**。本票收的是 md + 巨石 HTML → designing。 |
| `preview-by-construction.md` §2 | HTML playground = Android **layoutlib**；真机 = 产品 `HostSession`。designing **仍是 layoutlib 一侧**（意图静图），不得宣称 = 真机。 |

## 选定（锁进提案）

| 层 | 选定 |
|---|---|
| 壳 | bun 1.3 + Vite vanilla TS（`designing/app/`） |
| 预览 | cell-grid DOM + CSS 窗框 |
| Token | 保留 `DESIGN.md` frontmatter；生成器双写 playground + `designing/generated/` |
| 目录 | 数据/模块/生成物：`src/app/tui/designing/`；应用壳：`src/app/tui/designing/app/`（不平行 `tools/`，不进 `packages/xylitol-tui`） |
