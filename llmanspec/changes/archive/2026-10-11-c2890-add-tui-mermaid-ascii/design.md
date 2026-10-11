# Design — mermaid 围栏 ASCII

## 范围

本票只改 `packages/xylitol-tui` 的 Markdown 围栏路径。产品 host、thinking wrap、指针 / pan、产品配置都不改。助手正文已调用包 Markdown，闭合 mermaid 围栏会跟着生效。

## 决策

| id | 内容 |
|---|---|
| D1 | 默认 `MermaidFenceMode::Final`。`Off` 永不画。`Streaming` 对未闭合围栏也可尝试（失败回源）。 |
| D2 | `final` 用源字符串判定：trim 半截 closer 之后，若仍有未闭合的 mermaid 围栏（扫描 opener/closer，对齐 c2865），当代码。闭合后再走画图。不依赖 pulldown 的 End 事件（未闭合围栏 EOF 也会 End）。 |
| D3 | 语言：围栏 info 首词大小写不敏感等于 `mermaid`。 |
| D4 | 短路点：`render_code_block_lines` 在 `highlight_code` 之前。成功 → mermaid 结构块（盒线行 + 既有 indent，无标题框）；失败 → 红色提示 + **代码块内源码**（即使非法也原样，无围栏铬）；Off / `final`+未闭合 → 普通 markdown 代码块（无围栏铬、无提示）。不按 token 换图，避免流式跳变。 |
| D5 | 内置画图 = `mermaid-text::render_with_options`。它 `Ok` 则拆成行；`Err`（`EmptyInput` / `UnsupportedDiagram` / `ParseError` / `TooWide`）→ `None` 回源。不在本 crate 自研 layout。 |
| D6 | 调用方传入 **可用宽**（`render` 宽 − padding − indent）。`RenderOptions { max_width: Some(avail), max_width_strict: true, ascii: false, color: false, ..Default::default() }`。超宽是 crate 的 `TooWide`，本票再回源。盒线 **MUST NOT** 由 Markdown wrap。不用默认的软压缩（`render_with_width` 塞不下仍返回图）。 |
| D7 | 失败回源 = 错误色一行「无法渲染 mermaid」+ 代码块内源码（`theme.code` 行内代码色；源码比提示多一档 indent，prewrapped 保换行；即使非法也原样，无围栏铬）。**禁止** 把 mermaid 当 syntect 语言高亮（会塌成正文/muted）。成功画图 **MUST NOT** 再跟源码。`off` / `final` 未闭合不是失败，不加提示。 |
| D8 | `packages/xylitol-tui/Cargo.toml` 加 `mermaid-text = "0.57"`。不依赖 grok-mermaid、`@opencode/merman`、ratatui-markdown、termiflow、termermaid、mermaid-rs。 |
| D9 | `final` 未闭合围栏不调用 crate（当代码）。不采用 grok 的「丢最后一行再画」。crate 自己对流式半句的宽松解析，本票靠 D2 挡在门外。 |
| D10 | `MarkdownTheme::render_fence` 可选：`Fn(&str, Option<&str>, usize) -> Option<Vec<String>>`。`Some` 覆盖内置；`None` 或未设走内置。测注入用。 |
| D11 | 图画出来的行走既有 `code_block`。失败提示走 Palette `error`（`MarkdownTheme::mermaid_error`）。不新颜色 token。 |
| D12 | thinking 不经 Markdown，本票不画、不加开关。 |
| D13 | 不做 pan / 命中区 / `HitKind::Pan`。图是静态行。 |
| D14 | 拷贝 = 剥 ANSI 后的可见盒线。不另存 mermaid 源。 |

## 公开形状

```text
MermaidFenceMode { Off, Final, Streaming }   // Default = Final

Markdown::new(...)
  .with_mermaid_mode(mode)                   // 默认 Final
  .mermaid_mode()

MarkdownTheme {
  ...
  render_fence: Option<Box<dyn Fn(&str, Option<&str>, usize) -> Option<Vec<String>>>>
}
```

内置入口（crate 可见即可）：`try_render_mermaid(src, available_width) -> Option<Vec<String>>`，内部只包一层 `mermaid-text`。

## 布局

layout / 盒线字符 / sequence 列宽全部由 `mermaid-text` 负责。本票不复制 grok `layout.ts` 或 merman `DiagramCanvas`。

超宽：strict 预算失败 → 整块回源，不在 Markdown 里把盒线 `wrap_text`。

`identity_theme` / `Palette::markdown_theme`：`render_fence = None`，走内置。

## 输入 / 指针

无。本票不收键、不登记 HitRegion。

## 测试缝

复用 `Markdown::render(width)`。crate 单测喂围栏源看剥 ANSI 后的行。BDD 复用 `tests/bdd/steps_package_tui_markdown.rs`，不新开 harness。不测产品 host。

## 规格映射

| req | 行为 |
|---|---|
| r1947 | 已闭合 mermaid 能画则盒线、无围栏铬；失败则提示 + 代码块内源码 |
| r1948 | 默认 final：EOF 未闭合 mermaid 当代码；off 永不画 |
| r1949 | 自然宽超可用宽整块回源，盒线不折行 |

## 明确不做

- 图内 pan / sourceBox / mermaid.js / Kitty Image
- 产品配置、thinking 画图、跨端新动作 id
- 自研第二套 layout；整文件移植 grok-build `mermaid.rs`
- 依赖 ratatui-markdown / termiflow / termermaid / `@opencode/merman` / npm `grok-mermaid`
