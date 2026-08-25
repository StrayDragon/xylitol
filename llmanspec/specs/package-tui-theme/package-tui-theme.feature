# language: zh-CN
# capability: package-tui-theme
# purpose: 包内 SemanticPalette（Dark/Light）、paint 真彩、主题工厂与 demo 换肤。
# scope: packages/xylitol-tui/

功能: package-tui-theme

  @req:ptt01 @human
  场景: two-palettes
    - 包 MUST 提供恰好两套内建 SemanticPalette：Dark（对齐 DESIGN Mocha）与 Light（对齐 Catppuccin Latte）；MUST 能经 TerminalColorScheme 选取。

  @req:ptt02 @human
  场景: paint-truecolor
    - paint 辅助 MUST 用真彩 SGR 上色，前景复位 39、背景复位 49（或文档约定的行内恢复）；MUST NOT 要求组件依赖 JSON 主题文件。

  @req:ptt03 @human
  场景: theme-factories
    - 包 MUST 提供由 SemanticPalette 生成 MarkdownTheme 与 DiffTheme 的工厂；组件仍只收闭包主题。

  @req:ptt04 @human
  场景: demo-full-restyle
    - 当 agent_demo theme_mode 为 Light 时 MUST 将 markdown、diff、tool 背景与 muted chrome 切换为 Light 色板；Dark 时 MUST 使用 Dark 色板。

  @req:ptt05 @human
  场景: detect-opt-in
    - 终端色探测 MUST 仅在显式 opt-in（如 XYLITOL_AGENT_DEMO_THEME_AUTO）时启用；未启用时 MUST 保持 Dark。

  @req:ptt06 @human
  场景: host-feed-replies
    - 包 MUST 导出 OSC11 / CSI 色方案查询常量，并允许宿主将 reply 喂入 resolve_terminal_color_scheme；MUST NOT 因本变更强制产品 host 打开自动亮色。

  @req:ptt07 @human
  场景: thinking-border-palette
    - 包 MUST 提供 ThinkingBorderLevel（或等价）覆盖至少 off、minimal、low、medium、high、xhigh、max，并提供 cycle_next；Palette MUST 经 thinking_border_rgb（或等价）为每一 level 返回边框色，色阶 MUST 对齐 pi coding-agent theme dark/light.json 的 thinking*，且相邻 level MUST 可区分；MUST 提供 thinking_border_paint（或等价）返回真彩边框闭包；MUST NOT 依赖主 crate domain::ThinkingLevel；MUST NOT 用 muted/accent/warning 等通用 token 冒充 thinking 强弱。

  @req:ptt08 @human
  场景: paint-left-rail-line
    - 包 MUST 导出 paint_left_rail_line（或等价）：对给定行宽与轨色，输出「1 列轨底色空格 + 1 列无底色 gutter + 拟合内容」的真彩行；窄宽 MUST 不 panic（内容宽可压到 0）；轨段背景复位 MUST 遵循包 paint 约定（\\x1b[49m 或行内恢复）。MUST NOT 要求调用方手写轨/gutter 宽预算。
