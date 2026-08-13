# TUI 也是 App：稳定层 vs 可迭代层

> Change `c2200` 补充。2026-08-14。给「缺乏 TUI 设计经验、但懂 GUI」的人看。

## 1. 难想象的不是 TUI，是预览面错了

GUI 能迭代，是因为 Figma / Storybook / 真 App **画的是同一套像素近似**。TUI 的手写 HTML playground 更像「用 CSS 画 iOS」：丢掉格子宽、换行、差分绘制、时间（流式/折叠）、和弦（簇头 vs L1）。截图同样丢和弦。所以你会觉得「很难想象」——不是终端魔法，是 **没有对着真组件看状态**。

对的迭代面 = **真终端里的隔离场景**（生产 paint），外加给 agent 的语义 dump（纯文本树）。浏览器静图只适合看 token 色，不适合当 layout 真值。

## 2. 别人怎么拆层（一手/近一手）

**Claude Code（Ink/React）**
[源码教学 Ch.12 design-system](https://zhu1090093659.github.io/claude-code-cookbook/books/Chapter-12-Component-Library-and-Design-System.html)、[Ch.13 渲染](https://claude-code-from-source.com/ch13-terminal-ui/)：`design-system/` 只有 Dialog / Tabs / FuzzyPicker / ThemedBox 这类 **结构原子**；业务在 `messages/`、`permissions/`、diff。FuzzyPicker **泛型 + 回调画 item**，不搞中央「picker 模式注册表」。心智是 GUI 的原子设计。他们为 60fps 流式 **深叉 Ink**（typed array + cell dirty），说明「像 React 一样写」和「终端性能」要拆开：声明式组合 ≠ 用浏览器对象模型画格子。

**Codex CLI（ratatui）**
本仓 skill `tui-expert-of-codex`：`Renderable` + `desired_height` + Flex 两步分配（像 Flutter）；**stable scrollback + mutable tail** 管流式。没有独立 design playground；布局稳定性在 trait 和帧调度，不在 HTML。

**Pi（xylitol-tui 的上游心智）**
[`packages/coding-agent/docs/tui.md`](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/tui.md)：内置 Editor / SelectList / SettingsList / Loader；扩展用同一 `render/invalidate/handleInput`。主题 JSON。`DESIGN.md` 是架构还原文，**不是**视觉 SSOT。产品壳在 interactive-mode 大文件里——这正是 xylitol 刻意把壳留在 `src/app/tui`、引擎留在包里的原因。

**Charm**
Bubbles = 原子组件库；Lip Gloss = token/样式；[bubblebook](https://explore.market.dev/ecosystems/go/projects/bubblebook) = **终端里的 Storybook**（隔离跑 Model，不是浏览器画 TUI）。

**Textual**
用 CSS 描 TUI，最接近 GUI 设计师工作流；代价是另一套引擎。xylitol 已押差分 ANSI + Component，不必换。

## 3. 稳定什么、放开什么

映射 GUI：

| 层 | GUI | xylitol 现在 | 变更频率 |
|---|---|---|---|
| Token | 色/字/空 | `DESIGN.md` frontmatter → Palette | 低 |
| 原子 | Button / Input | 包：Editor、SelectList、Loader、Expandable、Markdown | API 低；内部用 `just test-tui` 改 |
| 壳 / 大布局 | App shell | status 0 行 idle、footer 1 行、队列条、toast、尾随提示 | **应稳定**（产品不变量） |
| 有机体 | 消息/工具卡 | `UiEntry` + ActivityFold 词表 | **高**——这是 activity-fold 反复修的层 |
| 页面状态 | Screen | 忙/闲、折叠开合、resume | 最高 |

「原子稳定、布局稳定、又能方便迭代」并不矛盾：

- **稳定** = 槽位契约（几行、落点、禁止顶插当默认）+ 原子 Component API。
- **迭代** = 新 transcript 形态只加一种贡献（计数/簇头/live id），加一条 Scene，不改壳、不改 Editor。

现在痛在有机体被写成闭世界 `match` + 把有机体 MUST 同时写进 `design/*.md`、playground、spec。壳和原子其实已经相对稳。

## 4. 对 xylitol 怎么做才对（建议，未废文档）

1. **想象工具**：`just` 跑 **产品 Scene**（真 TTY / VirtualTerminal），不要靠 HTML 脑补折叠。人类：进场景按键；agent：读语义 dump。
2. **Token** 继续一份数据（frontmatter 或生成 Palette+CSS）。HTML 若留，只当色板镜，不当 layout SSOT。
3. **原子** 继续在 `packages/xylitol-tui`；缺能力先改包。包 demo ≠ 产品目录。
4. **壳** 用短产品 spec（idle status 0 行）+ 测试，删掉绑 `mod.rs` 的句子。
5. **有机体** 走 ActivityAtom + Scene（c2200 难核 + 派工切片）。
6. **动效** 继续「高度一次揭开」；用帧带测哪一帧变、哪一帧禁止变。

不必等成为 TUI 设计师：用 GUI 同一套分层，把预览换成真组件场景。
