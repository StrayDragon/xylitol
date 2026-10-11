# Tasks

测试缝：`Markdown::render(width)`（围栏 lang 首词 `mermaid`）。crate 单测在 `packages/xylitol-tui`。BDD 复用 `tests/bdd/steps_package_tui_markdown.rs` + `bindings_package_tui_markdown`。不新开 harness。不测产品 host。

## T1 — 围栏短路与 final / 回源

- [x] `MermaidFenceMode::{Off,Final,Streaming}`，`Markdown::with_mermaid_mode`，默认 `Final`。
- [x] `MarkdownTheme::render_fence` 可选钩子；未设走内置。
- [x] `render_code_block_lines`：首词 `mermaid` 且模式允许时先试画；成功加既有 indent、不跟源。失败走 `mermaid_failed_paint`（红提示 + 源码比提示多一档 indent + `theme.code`）。Off / `final`+未闭合走 `highlight_code` 原路径、不加提示。
- [x] `xylitol-tui` 依赖 `mermaid-text = "0.57"`。内置 `try_render_mermaid` 只包 `render_with_options`（`max_width_strict: true`）。
- [x] 校验：crate 单测覆盖「闭合 TD 出盒线且无 \`\`\`」「未闭合 final 仍是源码」「Off 永不画」「钩子 Some 覆盖」。

## T2 — [blocked-by: T1] 超宽与其它能画的类型

- [x] `flowchart LR` 与 `sequenceDiagram` 各一条最小图经同一短路出盒线（crate 画，不自研 layout）。
- [x] `TooWide` / 可用宽不够 → 整块回源；盒线行不经 Markdown wrap。
- [x] 校验：crate 单测覆盖 LR、sequence 两参与者一条消息、窄宽回源。

## T3 — [blocked-by: T1, T2] BDD

- [x] `package-tui-markdown` 新场景绑到既有 `tests/bdd/steps_package_tui_markdown.rs` / `bindings_package_tui_markdown.rs`。
- [x] 校验：`cargo test --all-features --test bdd` 在本 change 分支相对 merge-base 绿（仅本 capability）。

## T4 — [blocked-by: T1] 文档与对照

- [x] `packages/xylitol-tui/PI_DELTAS.md` 一行：围栏 mermaid = `mermaid-text` 盒线，非 grok-mermaid / 非 Kitty Image。
- [x] `agent_demo` 可放一块已闭合 mermaid fence + 一块画失败回源（非产品 SSOT）。
- [x] designing `markdown` 补 drawn / too-wide / fallback-source 对照态（或 tui-lab 等价）；`just open-designing` 能打开。

不在本任务列 finalize。
