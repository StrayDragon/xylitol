# 代码即 design？TUI 验证与形态开闭

> Change `c2200`。2026-08-14。一手来源优先。散文 < 1500 词。

## 1. 本仓事实（闭世界）

`UiEntry` 是闭 enum（`src/app/tui/bridge/model.rs`）。`count_middles`（`activity_fold/summary.rs`）按变体 + 工具名字符串分类；未知变体 `_ => {}`（Todo 明确当投影、不算 Used）。`format_cluster_body` 用 if/else 优先级：Edited XOR Explored → Ran → Used → Asking → Thought → `"Activity"`。

c1762 教训（`.tmp/c1762-activity-fold-labels-lessons.md`）：和弦（簇头 vs L1）被截图混淆；N 的定义（去重名 vs 调用次数）抄错；live 标志绑全局流；时长未 persist。这些都是「按当前事实写分支」在下一形态到来时失真。

视觉栈：`DESIGN.md` token → `design/*.md` MUST → `playground/index.html`（2664 行，Agent 默认忽略，`design/AGENTS.md`）→ 产品 paint。`app-tui-chrome` `atc4` 把前两层写成 spec SSOT。包侧已有五层 harness（`package-tui-testing`）：键序列、insta、Clock、proptest、PTY/tmux。产品折叠仍大量靠 `harness.rs`（6321 行）与手测。

引擎已有 `Component::tick`（`packages/xylitol-tui/src/tui.rs`）与可注入 Clock（tt04）。产品 ActivityFold 仍用过全局 `Instant`/流字段（教训 3）。

## 2. 同类项目怎么做目录与验证

**Helix**（[docs/architecture.md](https://github.com/helix-editor/helix/blob/master/docs/architecture.md)、[compositor.rs](https://github.com/helix-editor/helix/blob/master/helix-term/src/compositor.rs)、[helix-tui TestBackend](https://github.com/helix-editor/helix/blob/master/helix-tui/src/backend/test.rs)）：`Component` + `Compositor` 层栈；**没有**浏览器 playground。验证 = in-memory `TestBackend` buffer diff。适合「作者即用户」的编辑器，不适合还要把固定态给非作者看的产品。

**Ratatui**（[官方 snapshot 食谱](https://ratatui.rs/recipes/testing/snapshots/)）：`TestBackend` + `insta` 把整屏当黄金。快，但单帧；不表达 t0→t1 关系。

**PTY 集成**（[ratatui-testlib / terminal-testlib](https://github.com/raibid-labs/ratatui-testlib)、[tuiwright](https://github.com/PandelisZ/tuiwright)）：真实 PTY + VT 仿真；cell-grid 快照；tuiwright 另有 asciinema 录像与 PNG。明确对比：`pexpect` 对全屏寻址 TUI 失效；vhs/asciinema 是演示不是断言；in-process backend 不练真实二进制。本仓层 5 已是这条路（`tests/tui_e2e/pty.rs`，`portable-pty`），且全部 `#[ignore]`。

**Widgetbook**（[widgetbook.io](https://www.widgetbook.io/)、[github.com/widgetbook/widgetbook](https://github.com/widgetbook/widgetbook)）：Flutter 的 Storybook。Use-case **就是**组件在隔离态下的目录；golden 从 use-case **生成**。Cloud 做 PR 视觉回归。**不**宣称废掉 Figma：Figma 仍给人看意图，代码目录给人/CI 看实现。1KOMMA5º 案例：隔离开发 + Cloud 拦视觉回归。

**OpenSpec 概念**（[Concepts](https://lzw.me/docs/openspec/en/concepts.html)）：spec = 可观察行为，不是 class 名。与本仓「产品级优先」同构；本票的「目录」应是可执行场景，不是第二份 MUST 散文。

## 3. 要不要废弃 design 文档和 playground？

**建议：不要一次性废光；要废的是「第二套手写实现」。**

| 方案 | 好处 | 代价 |
|---|---|---|
| A. 维持现状 | 人类开浏览器快 | 六层 SSOT；Agent 看不见静图；无帧；activity-fold 已漂移 |
| B. 全删，只留代码+测 | 单一真值 | 非作者无法扫色板/整壳；rustdoc 不渲染 TTY；review 只能跑测或真终端 |
| C. **代码驱动目录（推荐）** | 场景 = 测 = 可选生成 HTML/ANSI；token 仍一份数据 | 要建 Scene API 与生成器；迁移 playground 槽是体力活 |

C 对齐 Widgetbook：use-case 在代码里，浏览面是 **生成物**。Token 继续用 `DESIGN.md` frontmatter（或迁到一份 JSON/TOML 生成 Palette+CSS）——这已经是「数据即 design」，不是散文。`design/*.md` 应收成 **短意图**（和弦、禁止滑入、跨面同源），删除与 paint/spec 重复的编号 MUST 墙。`atc4` 必须改口，否则 spec 继续钉死文档路径（见 c2210）。

截图不够：教训 1 要求先看和弦再看像素。目录必须能 **dump 语义树**（L3 信封 / L2 簇 / L1 块），帧带断言「后开的 Thinking 不得把已封口 Thought 改回 Thinking」。

## 4. 形态开闭：穷举 match 其实是优点

Rust 穷举 match 在 **加变体** 时强迫处理。真正的坑是：

1. `_ => {}` 关掉穷举；
2. 工具角色用 `"edit" | "write" | …` 字符串，新工具名默默掉进 Used；
3. `ActivityCounts` 字段与词表优先级硬编码，新类目要改结构体 + formatter。

推荐：**投影期**把工具打成角色（Edit / Explore / Run / Used / Think / Ask / Compaction / Noise / Projection），fold 只消费角色。`UiEntry` 保持闭 enum（LSP 跳转好）。每个变体 `fn activity_atom(&self) -> Option<ActivityAtom>` 或等价表。新块 = 新 variant + 实现 atom + 一条 Scene。不要做运行时插件注册表（与产品「不是扩展市场」冲突）。

## 5. 帧带与性能

层 3 已禁止 `thread::sleep`。产品 tape 应：注入 Clock；每步 `idle_tick` + `render`；断言语义（簇头字符串、fold id）+ 可选 viewport 快照。PTY/tmux 只验协议与真终端，不验每条词表。

性能：已有增量摘要与「-3 不因打开簇 toggle 失效」（`design/activity-fold.md` MUST 15）。应变成 **可断言的 invalidate 计数**，而不是注释。多开占用：`lab_` 记 RSS/帧时；硬顶未测前不要写进 spec。

## 6. 对 xylitol 的含义

包 `Component` 已经可组合；缺的是 **产品形态的开闭** 和 **场景=目录**。`agent_demo` 继续当引擎 lab，不当产品 SSOT（现有 AGENTS 正确）。
