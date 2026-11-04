# Design — TUI 引擎能力接入：键位冲突、OSC8、性能面

## 目标

把三项低风险引擎能力接到产品：键位冲突提示、OSC8 链接、`/perf`；补 mermaid 护栏；清理零消费者 ActionRow。全部为增量，不改主交互语义。

## 现状（代码事实）

- 键位冲突：`KeybindingsManager::get_conflicts`（`crates/xylitol-tui/src/keybindings.rs:218`）；产品 `build_product_keybindings`（`crates/xylitol-app-tui/src/keybindings.rs:213`）已建 manager，但全仓无 `get_conflicts` 调用。`rebuild` 只统计 user-vs-user 冲突（`:170-187`）。
- OSC8：`hyperlink`（`crates/xylitol-tui/src/terminal_image.rs:11`）已导出；产品零引用。markdown 链接渲染 `label (url)`（`crates/xylitol-tui/src/components/markdown.rs:496-508`）；theme 只有 `link` / `link_url` 闭包（`:23-24`）。
- 性能快照：`RenderPerfSnap`（`crates/xylitol-tui/src/tui/render.rs:9`）；`last_render_perf`（`tui/mod.rs:636`）；产品只在 `lab_ao_perf.rs` / tests 用，无用户面。
- mermaid：`Markdown::new` 默认 `MermaidFenceMode::Final`（`components/markdown.rs:82`）；产品 `paint.rs:266` / `:384` 未显式设 mode；零产品 snapshot。
- ActionRow：`components/action_row.rs:47`；全仓零消费者（含 demo/tests）。
- 默认键位冲突：`app.clear` ctrl+c（`crates/xylitol-app-tui/src/keybindings.rs:14`）与引擎 `tui.input.copy` ctrl+c（`crates/xylitol-tui/src/keybindings.rs:111`）；默认表冲突不在用户覆盖时告警。

## 键位冲突提示

- 启动：`load_product_keybindings` 后取 `get_conflicts()`；仅当冲突涉及至少一个用户覆盖时提示（当前 manager 只统计 user-vs-user，天然满足）。
- 提示形态：耐久滚动提示（不是通知条）：`Keybinding conflict: <key> → <id1> / <id2>`；多条合并为一条，最多列 3 条。
- 触发点：TUI 首帧 ready 后一次；`/reload` 后再次检查并提示。
- 不阻塞、不退出、不改键位；用户可改 `keybindings.json`。
- 测试：用户把两个动作绑同一键 → 提示出现；无冲突 → 无提示；默认冲突 → 无提示。
- spec：`app-tui-input` 新规则「键位冲突提示」。

## OSC8 链接

- 引擎：`MarkdownTheme` 增加 `hyperlink_links: bool`（默认 false）；渲染链接时若 true → `hyperlink(label, url)`；否则现状 `label (url)`。
- 产品：`LayoutTheme` / `Palette::markdown_theme` 设 true；不新增配置。
- 兼容：不支持 OSC8 的终端忽略转义，仍显示 label；URL 不可见但可复制? OSC8 隐藏 URL 文本；若用户需要 URL，可关闭 hyperlink_links（暂不暴露配置）。设计取默认开。
- 测试：引擎单测 OSC8 序列；产品 snapshot 链接行含 `\x1b]8;;`；宽度计算不受转义影响（`visible_width` 已剥离）。
- spec：`app-tui-transcript` 新规则「链接可点」。

## /perf

- 命令：`/perf`；`PendingSlash::Perf`；host effect 读 `session.tui.last_render_perf()`。
- 输出：滚动提示多行：
  ```text
  perf: render 1.2ms · lines 585 · paint 120 · ao_reprojected yes · ao_shift yes
  ```
- 不新增面板 / 不常驻；只在用户触发时输出。
- 测试：命令解析、输出字段、无渲染快照时提示。
- spec：`app-tui-commands` 新规则「/perf」。

## mermaid 护栏

- 产品 `paint.rs` 构造 Markdown 时显式 `with_mermaid_mode(MermaidFenceMode::Final)`（若引擎无 setter，加一个 builder）。
- 产品 snapshot：一个 ```mermaid``` fence 的输出；断言不是裸 fence。
- 文档：DESIGN 或 transcript spec 注明 mermaid 默认 Final、无产品配置。
- 不改引擎默认，避免影响 demo。

## ActionRow 清理

- 确认 c3130 不使用 ActionRow 后，删除 `components/action_row.rs` 与 `lib.rs` re-export；同步删除测试。
- 若 c3130 选择用 ActionRow 承载 chip 行，则改为接线，不删。
- 二选一在 apply 开始时确认；默认删除（零消费者）。

## 实施任务

1. 键位冲突：启动 / reload 检查 + 滚动提示 + 测试。
2. OSC8：引擎 `hyperlink_links` + 产品设 true + snapshot。
3. `/perf`：命令解析 + effect + 滚动提示 + 测试。
4. mermaid：显式 mode + snapshot + 文档。
5. ActionRow：确认后删除或接线；同步 lib.rs / tests。
6. specs：`app-tui-input` / `app-tui-transcript` / `app-tui-commands`；`package-tui-*` 若需要。
7. 回归：`just test-tui`、相关 harness、`just qa`。

## 验证

- 键位：用户冲突提示；无冲突静默；默认冲突不提示。
- OSC8：产品 snapshot 含 OSC8；终端可点（真机 tmux / iTerm）；宽度不漂移。
- /perf：字段正确；无快照提示。
- mermaid：snapshot 不是裸 fence；引擎默认不变。
- ActionRow：删除后无编译引用；或接线 snapshot。
- 全量门禁。

## 风险 / 待确认

- 默认键位冲突（ctrl+c）是否应提示？设计取「仅用户覆盖冲突」；若用户希望默认冲突也提示，需另开。
- OSC8 在部分终端不可点；默认开可能让 URL 文本不可见；可后续加配置。
- `/perf` 在 release 也可用；是否只 debug？设计取都可用，成本低。
- ActionRow 删除是引擎公共 API 破坏；本包是 fork，允许；若外部嵌入方使用需保留（未确认）。
- mermaid 显式 Final 只是护栏，不改变输出。
