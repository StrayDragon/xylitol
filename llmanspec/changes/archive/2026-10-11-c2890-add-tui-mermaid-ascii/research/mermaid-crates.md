# mermaid TTY 盒线：pi / opencode / crates 一手对照

调研问题：xylitol-tui（MIT，`render → Vec<String>`，禁 `StyledLine`）画 mermaid 围栏时，能否 Cargo 依赖现成库，还是只能对照算法自研。

## 结论

唯一对得上许可证 + 字符串输出 + 硬宽度预算的 crate 是 **`mermaid-text` 0.57.0**（MIT）。本票改锁 Q2：Markdown 短路后调它，`Err` / `TooWide` 回普通代码块。

pi / `@opencode/merman` 都是 TypeScript，不能进 `xylitol-tui`。`termermaid` 是 grok-build 的 Rust 移植，但失败走 sourceBox、无硬宽度。`ratatui-markdown` 本地树是 SySL-1.0，crates.io 发布版也输出 `Vec<Line>`，两条路都禁。

## pi（`../pi`）

来源：`packages/coding-agent/src/modes/interactive/components/mermaid.ts`；依赖 `grok-mermaid` 0.2.3（`packages/coding-agent/package.json`）；默认 `markdown.mermaid: "streaming"`（`core/settings-defaults.ts`）。

- 只在 interactive TTY 装 Markdown transformer；首词 `mermaid`（大小写不敏感）。
- `render(token.text)` **不收宽度**；`!art || art.width > availableWidth` → **`token.raw`（整段围栏原文）**。pi **不调用** `sourceBox`。
- thinking / `off` / (`isStreaming && mode !== "streaming"`) 原样返回。
- 非流式且 `art.warnings.length > 0`：丢图，保留 `token.raw` 另附警告行。
- `grok-mermaid`：Apache-2.0 TS 港；`null` = 空/未知类型/过大（`MAX_NODES=128` 等）；严格图会丢最后一行再试。xylitol **不能 Cargo 依赖**。

## opencode（`../opencode/packages/merman`）

来源：`package.json` `name: @opencode/merman` private；仓库根 `LICENSE` MIT。TUI 经 `registerCodeBlockRenderer("mermaid", …)`（`src/plugin.ts` / `src/markdown.ts`）。

- 手写 TS 解析 + `DiagramCanvas`；输出 OpenTUI `StyledText`，不是 ANSI `Vec<String>`。
- 类型：flowchart / sequence / state / timeline / gitGraph / gantt。未知或画布 > 250_000 格 → `undefined` → 普通代码围栏。
- 默认 `layoutMaxWidth` 120 再 `min(configured, ctx.width)`；宽 sequence **水平 pan**（`scrollX`）。本票不做 pan。
- crates.io 的 `merman-core` / `merman-ascii` / `mermansi`（Latias94 / kingsword09，MIT OR Apache-2.0）**不是**这个包；opencode 无 `Cargo.toml`，未用这些 crate。

## crates.io（2026-10-11 `cargo info` + registry tarball）

| crate | 许可 | 输出 | 宽度 | 对本票 |
|---|---|---|---|---|
| **mermaid-text 0.57.0** | MIT，`rust-version` 1.92（低于 xylitol 1.99） | `Result<String, Error>` | `RenderOptions { max_width, max_width_strict }` → 超宽 `Error::TooWide` | **采用**。依赖 `unicode-width`（包已有）+ `ascii-dag` + `chrono`。无 ratatui。tarball 无 LICENSE 文件，crates.io 字段 MIT。 |
| termermaid 0.5.0 | Apache-2.0 | `Option<String>`；README 的 `termermaid::render` 与 crate 根导出不符 | 无硬预算 | 失败 boxed source；~90 dl。**grok-build 不用它**（它从 grok 抠出）。方括号标签实测丢边。不用。详见 [termermaid-vs-mermaid-text.md](./termermaid-vs-mermaid-text.md)。 |
| termiflow 0.2.8 | MIT | `Result<String>` | 有 | **ratatui**。不用。 |
| mermansi 0.1.6 | MIT/Apache | 文本；默认可附语义 JSON | `with_max_width` | 钉 `merman-*-0.8.0-alpha.3`，rust-version 1.95。过重。 |
| merman-ascii 0.8.0 | MIT/Apache | 要 `merman-core` 模型，不直接吃源字符串 | OverflowPolicy | 面太大。 |
| meraid 0.2.0 | MIT | `anyhow::Result<String>` | 无列预算 | lib 拉 clap。弱。 |
| console-mermaid 0.1.0 | MIT | `Result<String, String>` | padding 非列预算 | 28 dl。弱。 |
| mermaid-rs 0.1.1 | — | SVG | n/a | `headless_chrome` + mermaid.js。禁。 |

本地 `../ratatui-markdown/LICENSE` = **SySL-1.0**（禁进 MIT 仓）。crates.io `ratatui-markdown` 0.3.6 标 MIT OR Apache-2.0，但 mermaid API 是 `Option<Vec<ratatui::text::Line>>`，撞 D12。两条都不用。

`mermaid-text` 入口（tarball `src/lib.rs`）：`render` / `render_with_width`（软预算，塞不下仍返回最紧凑图）/ `render_with_options`。本票必须 `max_width_strict: true`，才能把超宽变成 `TooWide` 再回源（对齐 pi 比宽、对齐 r1949）。默认软压缩会画出超宽行，与「盒线不折行、超宽回源」冲突。

## 依赖重量（plan B：grok-mermaid）

2026-10-11 `cargo tree` / `package.json`：

| 实现 | 运行时依赖 | 接入 xylitol-tui |
|---|---|---|
| **mermaid-text 0.57** | `ascii-dag` + `chrono`（`iana-time-zone`/`num-traits`）+ `unicode-width`（包已有）→ **7 个 crate** | Cargo 直接依赖。chrono 只为 gantt 时钟。 |
| **grok-mermaid 0.2.x** | **零** npm `dependencies`（12 个 TS 文件，自带 width 表） | **不能 Cargo**。要 Node/Bun、WASM，或整文件迁 grok-build `mermaid.rs`。 |
| **termermaid 0.5** | 仅 `unicode-width`（更瘦） | 旧 grok 盒线的**有回归**港。现树 grok-build **Cargo.lock 无此 crate**。失败 **sourceBox**、无硬宽度、`A[Start]` 丢边。见 [termermaid-vs-mermaid-text.md](./termermaid-vs-mermaid-text.md)。**不是** plan B。 |

grok-mermaid 本身不「依赖多」，多的是 **运行时形态**（JS）。本票用 mermaid-text；若盒线观感不可接受，下一票评 **grok-build `mermaid.rs` 裁剪**（剥 ratatui、超宽改 `None`），不要再试 crates.io termermaid，也不要在 TUI crate 里起 Node。

`render_with_width` 对 sequence/pie/er 等是固定布局，软预算可能被忽略；strict 仍可在事后量宽拒收。

## 明确不采用

- npm `grok-mermaid` / `@opencode/merman`：运行时不是 Rust。
- 整文件迁 `grok-build/.../mermaid.rs`（现树 **5699** 行 Apache-2.0 + ratatui）：切片过大。现树另有 PNG 栈 `xai-grok-mermaid`（resvg/dagre/子进程），撞 D11，不跟。
- 自研小网格：在已有 MIT 字符串库之后重复 layout/A\*，apply 风险更高。
