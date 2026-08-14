# 派工 prompt：TUI `designing` Web 应用（提案 + 可维护骨架 + 首模块迁移）

把 **下面从「环境」起的整段** 交给另一个 session。本票是 **设计文档 / 静图 SSOT 改良**，**不是** 产品 TUI PreviewInject / HostSession-in-browser / Scene 框架。

目标一句话：把今天的 `DESIGN.md` + `design/*.md` + `playground/index.html`（三源、易脱轨）收成 **两个真值**——**(1) 产品代码** 管运行时；**(2) `designing/` 模块化 Web 应用** 管意图 + 结构化静图 + tokens。人类在浏览器里预览；Agent 只读短结构化上下文，不吞 2.6k 行 HTML。

---

## 环境（硬）

独立 git worktree，**不要**在主仓脏 `main` 上改（主仓可能同时改 PreviewInject / 死码 / specs）。

```bash
cd /home/l8ng/Projects/__straydragon__/xylitol   # 仅用来创建 wt
wt switch --create --no-cd -y c2230-tui-designing
cd "$(wt list --porcelain 2>/dev/null | awk '/c2230-tui-designing/{print; exit}')"  # 不行就 wt list 看路径
# 常见路径：../xylitol.c2230-tui-designing
eval "$(just cargo-wt-env)"   # 若本票改 Rust Palette；纯 bun 也建议执行，避免以后误用主仓 target
```

分支名：`c2230-tui-designing`。**禁止** ff 进 main。**禁止**在默认分支改 `llmanspec/specs/**`（atc4 只出改写草稿，落地可另开或交给 c2210）。

本仓产品是 **Rust 2024 个人 coding agent TUI**。用与用户相同的语言（中文）写提案与 AGENTS。提交 Conventional Commits；不加 co-author。

需要 bun：确认 `bun --version`。没有就装，不要改成 npm 当默认。

---

## 先读（按序，不要跳）

1. `src/app/tui/design/AGENTS.md` — 今天的人/Agent 分界、playground「唯一静图 SSOT」句（本票要改掉这句）
2. `src/app/tui/DESIGN.md` — YAML frontmatter = **现有 token SSOT**（颜色/字号/spacing/components）
3. `src/app/tui/design/playground/README.md` + `playground/index.html` 头 120 行 + 槽目录（约 2664 行，**不要整文件当上下文反复读**）
4. `src/app/tui/design/fixtures/*.yaml` — 已有状态夹具（`must_contain` / `slot`）
5. `scripts/check_tui_design_playground.py` + `src/app/tui/design/playground/sync_tokens.py` + just：`sync-tui-tokens` / `check-tui-tokens` / `open-design-playground`
6. `docs/architecture/TUI信息面与chrome词汇.md` — 产品用词（下轮预告 / 滚动提示 / 壳层通告…）
7. `llmanspec/specs/app-tui-chrome/spec.toon` 的 **atc4**（钉了 design 路径当视觉 SSOT）
8. 调研（只读，**不要**按 C 方案去实现 HostSession 目录）：
   - `llmanspec/changes/c2200-refactor-tui-kind-catalog-verify/research/code-as-design-and-tui-verify.md` §3
   - `preview-by-construction.md` §2（HTML playground = Android layoutlib；真机仍是产品 host）
9. 根 `AGENTS.md` Pre-0.0.1 卫生：禁止兼容别名 / 双路径。改名 = 一次性改调用点。
10. `src/app/tui/AGENTS.md` — `agent_demo` ≠ 产品 SSOT（保持）

抽读 1～2 个组件体会脱轨形态：`activity-fold.md`（MUST 墙很长）vs `fixtures/activity-fold.*.yaml` vs playground 槽 `activity-fold`。

---

## 问题（今天为什么痛）

三套「看起来像设计」的东西：

| 源 | 给谁 | 问题 |
|---|---|---|
| `DESIGN.md` frontmatter | 人 + 闸 + `Palette` | 还行；是真正的数据 |
| `design/*.md`（~29 个） | 人 + Agent | 编号 MUST 墙；钉实现细节；Agent 当 SSOT 读会过时 |
| `playground/index.html`（~2664 行） | 人（浏览器） | **Agent 默认忽略**；手写第二套 UI；与 md/产品脱轨；不可模块化 |

再加上产品代码 = 第四源。本票 **不**把产品 paint 搬进浏览器；只把 **md + 巨石 HTML** 收成一个可预览、可模块化、对 Agent 便宜的 `designing`。

成功标准（可观察）：

- 人类：`bun run dev`（或 `just open-designing`）能按 **组件模块** 预览固定态；色/尺寸来自结构化 token，不是槽里手写 hex。
- Agent：改某一组件时默认只读该模块 `intent.md` + `states/*.yaml` + 生成的短 index；**禁止**把整站 HTML/bundle 当上下文。
- 闸：tokens 仍与 Rust `Palette` 对齐；静图夹具闸从「解析 2664 行 HTML」逐步改到「模块 states」。
- 源数量：视觉意图 **2**（`designing` + 产品代码），不再是 3（md + html + 代码）。`DESIGN.md` 正文可缩成 designing 的 Overview 或生成物。

---

## 非目标（硬，违反即偏题）

- **不要** 做产品 TUI 的 Web 运行时：不 wasm `xylitol-tui`、不嵌 `HostSession`、不 PreviewInject、不 `/debug` 侧栏、不 tui-pantry。
- **不要** 宣称浏览器预览 = 产品真值。格子比、差分绘制、鼠标、流式帧仍以产品 host / harness 为准。designing 是 **意图 + 固定态对照**。
- **不要** 把 `agent_demo` 收进 designing。
- **不要** 一次删光 `design/*.md` 和 `playground/index.html`。先双轨：新模块绿 → 再迁 → 最后删巨石。Pre-0.0.1 **允许**短双轨，但 **必须**在提案里写清拆除条件（哪条闸切过去就删旧文件），禁止无限期兼容。
- **不要** 为未交付能力堆抽象（通用 Scene、插件市场、Figma 同步）。
- **不要** 在默认分支改 live specs。atc4 只在提案里给产品级改写句。

---

## 调研（必须先写进 research，再锁依赖）

交付文件：`llmanspec/changes/c2230-add-tui-designing-web/research/stack-survey.md`

用表，不要散文堆。每类 **≥3 候选 + 1 推荐 + 否决理由**。用现网文档（context7 / 官方），不要凭训练记忆锁版本。

### A. 设计应用壳（给人浏览模块的 UI）

要：bun 一等；启动快；适合 **文档+状态目录**，不是 React 组件车间。本应用的「组件」是 TUI 表面（activity-fold、toast…），不是 Button/Dialog。

候选至少评估：

| 候选 | 看什么 |
|---|---|
| **自研 bun + Vite（或 bun --hot）+ 少量 UI** | 控制力、token 预算、零 Storybook 心智 |
| **Ladle** | Vite、CSF、轻；是否把我们锁进 React Story 模型 |
| **Storybook** | 生态大；对「终端静图」过重，默认否决除非写出不可替代点 |
| **Histoire** | Vue/Svelte；本仓无此栈则倾向否 |
| **VitePress / fumadocs / Starlight** | 文档站；预览槽是否别扭 |
| **shadcn / Base UI / Park UI** | **只**当 designing **壳**的控件，不当 TUI 像素 |

推荐原则：**壳可以丑，模块目录必须清**。优先自研小壳，除非调研证明 Ladle 能 **直接吃 YAML states** 且不逼我们写 React TUI。

### B. 终端「看起来像 TTY」的渲染

要：固定态可预览；人类能感到格子/轨/折叠；**结构化数据可 round-trip**（Agent 能读 states，不必 OCR 屏幕）。

候选至少评估：

| 候选 | 看什么 |
|---|---|
| **结构化 cell-grid**（JSON/YAML → DOM 等宽格子，1 cell） | 最可维护、token 最省、可闸 `must_contain`；不像真终端 |
| **xterm.js** (`@xterm/xterm` + headless) | 成熟；canvas 对「选中/搜索/Agent 读 DOM」不友好；易诱人喂 ANSI 当设计源 |
| **Vercel wterm** | DOM 终端、可访问性；Zig/WASM 重不重；是否适合 **静图** 而非 PTY |
| **@termless/** / ghostty-web | 真 VT；对本票静图过重 |
| **asciinema player** | 录像不是设计模块 |
| **纯 CSS 仿终端窗口**（chrome only） | 壳可以；**内容**不要靠 CSS 手画第二套 layout |

**调研必须回答的题：**

1. 设计 **源数据** 应是 cell/span 树、ANSI、还是 HTML 模板？
2. 仿真器是 **预览器** 还是 **作者格式**？（本票强烈倾向：源数据 ≠ xterm；xterm 最多当一种 view）
3. 80×24 vs 窄宽态如何在模块里声明（已有 models.narrow / pending fixtures）？

**预置倾向（可被证据推翻）：** 作者格式 = YAML/JSON 状态 + 短 intent；预览 = cell-grid DOM（默认可检）；TTY 窗框用 CSS。只有当 cell-grid 无法表达某 MUST（例如真 SGR）才加 xterm **只读 view**。

### C. Token / 结构化尺寸颜色

已有：`DESIGN.md` YAML frontmatter → `tokens.css`/`tokens.js` + Rust `Palette`（`sync_tokens.py`）。

评估：W3C Design Tokens JSON + Style Dictionary vs **保留 frontmatter、只改生成器输出到 designing**。不要无故引入 Tokens Studio/Figma。

硬约束：

- **一份** 颜色/spacing 数据；禁止 playground 与 DESIGN 各写 hex（现闸已禁，迁过去仍禁）。
- 组件模块可 **引用** token（`{colors.muted}`），不可另立冲突 hex。
- 改 token 必须继续能 `just check-tui-tokens`（可改脚本路径，不可丢闸）。

### D. Agent 上下文（token 友好）

设计并写进 `designing/AGENTS.md`：

| 受众 | 默认读 | 禁止默认读 |
|---|---|---|
| Agent 改某表面 | `modules/<id>/intent.md`（软顶 ~80 行）+ `states/*.yaml` | `app/` 源码、生成 CSS、整包 HTML、node_modules |
| Agent 找路 | 生成的 `generated/AGENT-INDEX.md`（每模块一行：id、一句话、states 列表） | 29 个旧 md 一次性灌入 |
| 人类 | 浏览器 designing + 偶尔 intent | — |

生成 index 的命令要进 just（如 `just gen-designing-index`），改模块漏跑要能被 qa 抓到。

intent.md 只写 **可观察 MUST**（词表、禁止滑入、轨/flush、跨面同源）。禁止钉 Rust 路径/类型/行数（产品级 spec 纪律）。

---

## 建议的仓库形状（调研后可微改，目录职责不要散）

倾向把 `src/app/tui/design/` + `playground/` **演进为**：

```text
src/app/tui/designing/
  AGENTS.md                 # 人/Agent 分界；本票必写
  tokens/                   # 从 DESIGN.md 迁入或生成；唯一色板数据
  app/                      # bun Web 应用（壳：导航 + 预览窗）
  modules/
    activity-fold/
      intent.md
      states/collapsed.yaml
      states/envelope.yaml
      preview.ts            # 只组装该模块的 cell/span，禁止 2k 行上帝文件
    chrome-toast/
    ...
  generated/                # 生成物：AGENT-INDEX.md、tokens.css；勿手改
```

若 bun 应用放在 `src/app/tui` 下会脏了 crate 布局，可改为：

- 数据/模块：`src/app/tui/designing/`（靠近产品面，Agent 找得到）
- 应用壳：`tools/tui-designing/` 或 `src/app/tui/designing/app/`

**选定后写进提案，不要两套。** `packages/xylitol-tui` **不**平行维护 design HTML（现规则保持）。

现有槽（迁移清单，提案里打勾，本切片不必全迁）：

Full shell · Layout · Keybindings · Models · Pending · Chrome toast · Mcp · Tree · Resume · Compaction · Activity fold · Tool · Diff · Markdown · Palette · Widgets · Atoms · Ask

对应 md：`design/*.md` 约 29 个。`session-tree-vs-pi.md` 是对照文，可留 research 或缩进 intent。

---

## 本 wt 要交付的（有序，做完可停）

### 1. 调研 + 提案

- `llmanspec/changes/c2230-add-tui-designing-web/proposal.md`（purpose-draft 可先，但本 wt 结束前要能指导实现）
- `research/stack-survey.md`（A/B/C/D 表 + 推荐）
- 提案必须写清：
  - 两源模型（designing vs 产品代码）
  - 浏览器预览 **不是** 产品真值
  - 旧 playground / 旧 md 拆除条件
  - atc4 产品级改写草稿（不落 specs）
  - 与 c2200 PreviewInject **正交**：那边管真 host 夹具；这边管设计意图静图

### 2. bun 应用骨架（可运行）

- `package.json` / `bun.lock` 只在 designing 应用目录；不要污染主 crate。
- `bun run dev` 能打开：左模块列表，右固定态预览，token 驱动颜色。
- `just open-designing`（或改名替换 `open-design-playground`，旧命令可暂时转发，提案写拆除日）。
- 壳 UI 克制：不要顶栏快捷键墙；不要用「(包)」分层样式（沿用现 playground AGENTS 硬约束）。

### 3. Token 管道

- 一份数据喂：designing CSS + 现有 Rust `Palette` 检查。
- 现 `just check-tui-tokens` **必须仍绿**（可改 `sync_tokens.py` 输出路径）。
- 禁止手改生成物。

### 4. 首个模块迁移（推荐 `activity-fold`）

原因：MUST 墙最肥、已有 fixtures、最容易证明「模块化 > 巨石 HTML」。

- `intent.md`：从 `activity-fold.md` **压缩** 可观察句；删实现组织；保留词表（Thinking / Thought Ns / Used N=调用次数 / 禁止 Planning next moves…）。
- `states/`：移植现有 `fixtures/activity-fold.*.yaml` 的 `must_contain` / `must_not_contain`。
- 预览：collapsed / envelope / expanded 至少两态可点切换。
- 闸：该模块的 fixture 检查走 YAML states，不再从 `index.html` 抠 template（旧脚本对其它槽可暂时仍解析 HTML）。

### 5. Agent 阅读体验

- `designing/AGENTS.md` 取代（或明确 supersede）`design/AGENTS.md` 里过时的「唯一视觉 SSOT / Agent 忽略 HTML」。
- `generated/AGENT-INDEX.md` 生成 + qa 检查过期。
- 给 Agent 的默认路径写进根或 `src/app/tui/AGENTS.md` **一小段指针**（不要把长教程搬进根 AGENTS）。

### 6. 验证

```bash
eval "$(just cargo-wt-env)"
bun run --cwd <designing-app> check   # 你加的 typecheck / 生成物 check
just check-tui-tokens
python3 scripts/check_tui_design_playground.py --check   # 未切完全闸前必须仍绿
# 若动了 Palette / Rust：
cargo test --lib -- palette   # 或现有 token 相关测
```

能跑 `just qa quiet` 更好；至少不要让旧闸红。新 check 脚本若叫 `scripts/check_*.py` **必须**接到 `just qa`。

---

## 迁移策略（本 wt 内写进提案，代码只做第 4 步那么深）

1. **骨架 + tokens + activity-fold**（本 wt）
2. 其余槽按依赖迁：tokens/palette → chrome 词表（status/footer/toast/pending）→ transcript/expandable → pickers（models/tree/mcp）→ 整壳
3. 旧 `index.html` 每迁走一个槽就删对应 JS 模板，禁止复制粘贴成第三份
4. 全绿后删 `playground/index.html`，`just open-design-playground` → alias 到 designing
5. 旧 `design/*.md`：已迁的改成「已迁 designing/modules/…」一行指针然后删正文；或直接删（Pre-0.0.1 无 SemVer 读者）

---

## 提交

- 调研：`docs(sdd): survey bun designing stack for tui intent preview`
- 骨架：`feat(tui): add designing web app shell and token pipeline`
- 首模块：`feat(tui): migrate activity-fold intent into designing module`

PR/说明对照本 prompt 的交付 1–5。不要和 c2200/c2210/c2220 混一个 PR。

---

## 完成时回报（给架构师的短文）

1. 选了哪套壳 / 哪套预览渲染 / 哪套 token，各用 **三行** 证据。
2. 目录最终落点。
3. Agent 默认读哪些文件（列路径）。
4. 旧 playground 还剩什么、拆除条件。
5. 未做项（其余槽、atc4 落 specs、xterm view…）。
