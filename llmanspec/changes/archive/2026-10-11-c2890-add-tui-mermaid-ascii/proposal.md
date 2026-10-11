---
depends_on: []
branch: sdd/c2890-add-tui-mermaid-ascii
base_branch: main
base_sha: 173f646a847e3c75cbd503e2df59b02409e53847
needs_specs_change: false
---

# Markdown 围栏内 mermaid 终端 ASCII 图

## Why

pi `grok-mermaid` 与 opencode merman 在 TTY 画流程图。xylitol 把 `lang=mermaid` 当普通代码高亮。D11 裁掉 Kitty Image——本票只做 Unicode 盒线，不碰协议位图。

**不依赖** ratatui-markdown：输出是 `Vec<Line>`（撞 D12），许可证 **SySL-1.0**（AI 披露 copyleft，不能进 MIT 仓）。可对照其「layout 自定宽、调用方比可用宽、失败回源」契约，**禁止**依赖或整文件搬 pest grammar。

图内平移（merman 可水平拖）需要 c2858 `HitKind::Pan`，**本票不做 pan**。半截围栏高度与 c2865 相关——默认 `final` 可解耦。

## What Changes

- 包 `Markdown`：围栏语言首词为 `mermaid` 时走终端流程图（无标题框），不是普通代码高亮。`mermaid-text` `Ok` → 只出图、不跟源；`Err`（含 `TooWide`）→ 红色「无法渲染 mermaid」+ **代码块内源码**（行内代码色；源码比提示多一档缩进，即使非法也原样，无围栏铬）。**MUST NOT** `sourceBox`。
- 模式默认 **`final`**，避免流式每 token 换图跳变：EOF 未闭合的 mermaid 围栏仍当代码（跟其它围栏一样长高）；闭合成功才换成结构块。`streaming` 才对未闭合尝试画图。`off` 永不画。
- thinking 正文走 plain wrap、不经 Markdown，本票不另开开关。
- 拷贝 = 可见字符：画成图就粘贴艺术字，不承诺「拷贝仍是 mermaid 源」。
- 产品助手正文已走包 Markdown，默认继承；本票不加产品配置项、不改 host。
- 可执行合约 SSOT：`packages/xylitol-tui/llmanspec/specs/package-tui-markdown/`。根 specs-landed 只扫 `llmanspec/specs/`，故标 `needs_specs_change: false`。

## 非目标

- 不复活 D11 / mermaid.js / WebView。
- **不**依赖或搬迁 `ratatui-markdown` / `@opencode/merman` / npm `grok-mermaid`；**不**把 grok-build `mermaid.rs`（~5k 行）整文件迁入。
- 不保证 mermaid.js 全语法。画图只经 `mermaid-text`；它报错则回源。不在本 crate 自研第二套 layout。
- **不做** 图内 pan/zoom（将来另票，`depends_on: [c2858]`）。
- 不改 thinking 画图；不加产品 `/config` mermaid 项。

## Capabilities

- `package-tui-markdown`（嵌套包实例）

## Impact

- 与 c2865：默认 `final` 则流式半图不抖动；闭合围栏那一帧才换成图（一次高度跳变可接受）。
- 产品 `paint_assistant_block` / 流式助手已调用包 Markdown，闭合 mermaid 围栏会自动出现盒线。
- `agent_demo` 可放一块 fence，不是产品 SSOT。
- 超宽回源后 wrap 缓存与 Diff SBS 阈值不变（仍是源码行）。

## Open Questions

1. ~~默认 `final` vs pi `streaming`？~~ **`final`**（2026-10-11）。c2865 D6 已按此解耦；streaming 每 token 重 layout 会抖。
2. ~~自研网格 vs 重写 grok-mermaid？~~ **Cargo 依赖 `mermaid-text` 0.57.0**（2026-10-11 深挖后改锁）。MIT，`render_with_options` + `max_width_strict` → `TooWide` 回源。不自研 layout，不搬 grok-build `mermaid.rs`，不依赖 grok-mermaid / `@opencode/merman` / ratatui-markdown / termermaid。
3. ~~失败是否用 `sourceBox` 标题框？~~ **否**（2026-10-11）。回退代码块内源码（无围栏铬），不是标题框；成功画图仍无围栏铬。
4. ~~thinking 要不要画？~~ **不要**（2026-10-11）。thinking 不经 Markdown。
5. ~~本票产品配置？~~ **不要**（2026-10-11）。默认 final 随包 Markdown。

## Further Notes

- [research/mermaid-peers.md](./research/mermaid-peers.md)
- [research/mermaid-crates.md](./research/mermaid-crates.md)（pi / opencode / crates.io 一手；Q2 改锁依据）
- [research/termermaid-vs-mermaid-text.md](./research/termermaid-vs-mermaid-text.md)（2026-10-11 现树 `2bdd1d6a` 复读）：**grok-build 仍不用 termermaid**。双栈 = 仓内 `mermaid.rs` 5699 行 inline 盒线 + `xai-grok-mermaid` 点按 PNG（Warp `mermaid-to-svg`）。termermaid 是旧盒线的有回归港。保持 mermaid-text；PNG 栈撞 D11。
- pi 失败回 `token.raw`（整段围栏），**不用** `sourceBox`。xylitol 失败回代码块内源码（提示行在上，无围栏铬）；成功画图与普通代码块仍遵守 r1645。
- 一手路径：`../pi/packages/coding-agent/src/modes/interactive/components/mermaid.ts`；`../opencode/packages/merman`；`../grok-build/crates/codegen/xai-grok-markdown/src/mermaid.rs`；registry `mermaid-text-0.57.0` / `termermaid-0.5.0`
