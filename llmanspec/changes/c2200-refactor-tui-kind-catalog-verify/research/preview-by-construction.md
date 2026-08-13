# 鸿沟：看见 · 锁住 · 同源 · 能改

> c2200。给「TUI 也是 App、但很难想象」的统一设计，不是新渲染引擎。

## 1. 你要的四件事其实是同一物件的四个用法

| 需求 | 失败时 | 对的载体 |
|---|---|---|
| 新组件看见 **设计 + 交互** | HTML 静图 / 截图丢和弦与按键 | 真 `HostSession` 上按键 |
| 已固定组件 **锁 UI** | 散文 MUST、playground 漂移 | Frozen 基线（屏 + 语义树）合入要接受 |
| **和业务同源** | demo / HTML / 手搓 paint | 与生产同一 `apply_ui_model`→`UiRoot::render` |
| **还能迭代** | 改一处动五处 | 夹具一份；Evolving 不进视觉闸 |

不是四套系统。是 **一份 Preview 夹具** 的四种消费：人看、人点、CI 锁、代码改。

## 2. GUI 界已经踩过的坑（不要学错的那边）

**SwiftUI `#Preview { PostCard(post: .preview) }`**
表达极简，就写在组件旁边。Xcode canvas 仍可能慢/脆；复杂组件要 mock。价值是 **心智：Preview 是组件的一部分，不是隔壁 HTML。**

**Compose `@Preview`**
[HotSwan：Preview ≠ 真机](https://hotswan.dev/blog/compose-preview-vs-device)：Studio 用 **layoutlib 在桌面 JVM 仿真** Android，字体/色/间距会偏。他们的解法是 Preview **跑到真机/模拟器** 上——同源优先于「IDE 里看起来快」。

TUI 的 HTML playground = layoutlib。格子 1:2、差分绘制、鼠标、流式，仿真都会撒谎。**「真机」= 产品 `HostSession` + xylitol-tui render。**

**Storybook + Chromatic / Widgetbook Cloud**
Story 同时是目录、交互台、视觉测输入。Frozen = 接受后的 baseline；PR 上像素 diff。Widgetbook 明确：**动画会让视觉对比坏掉**，要用 knob 关掉再拍。TUI 对应：动效用 **帧带 + MockClock**，不要对 spinner 做 insta 黄金帧。

**Compose Preview 的教训**：仿真预览会漂。我们若做目录，右侧必须是真渲染，不能是「看起来像终端的 CSS」。

## 3. 统一物件（够用、能自省、可审查、好表达）

不要新框架名。扩已经存在的 `/debug` 目录（`src/app/debug_fixtures/catalog.rs`）。

每个可预览面是一条 **Preview**（就写在该组件/夹具旁）：

```text
id:          "activity-fold.thought-only"
stability:   Evolving | Frozen
mount:       把 HostSession 放到该态（XyEvent tape 或 seed UiModel）
keys?:       可选按键带（看交互）
invariants:  语义树断言（L3/L2/L1、N 的定义、live id）
```

四种消费同一条：

| 消费 | 做法 |
|---|---|
| 看见 | `just tui-preview` 或产品内 `/debug <id>`：真 TTY，可按键 |
| 锁住 | `Frozen` → insta 屏（去 spinner）+ 语义 dump；PR 要人接受 diff |
| 自省 | 编译期/测：每个 `UiEntry` 变体、每个 chrome 槽至少一条 Preview（inventory） |
| 表达 | 十来行 Rust 或 YAML seed；rust-analyzer 能跳到 `mount` |

**C 方向（pantry 侧栏）** 是这种物件的 **浏览器 UI**：左 SelectList 列 Preview，右 **嵌同一个 HostSession**。自研 example，不引进 tui-pantry/ratatui。先 `/debug` 列表也能活；侧栏是体验升级不是架构前提。

稳定性怎么不丢特性：

- `Frozen`：视觉+语义进 CI；改外观必须改基线（审查点）。
- `Evolving`：只进目录和交互，不挡合并。
- 不变量（「封口 Thought 不被后来 Thinking 改写」）写在 Preview 上，不写在 `design/*.md` 编号墙。
- ActivityAtom 保证 **新变体不能不登记**；Preview inventory 保证 **新变体至少能被看见**。

## 4. 刻意不统一的

| 层 | Preview 挂哪 | 为什么 |
|---|---|---|
| 包原子（Editor…） | `xylitol-tui` tests / 可选包 example | 无 `XyEvent` |
| 产品壳+transcript | `HostSession` + debug catalog | 这才是业务同源 |
| 真终端协议 | 现有 PTY，极少 | 不进日常目录 |

`agent_demo` 继续只验引擎。产品 Preview **禁止** 画 demo 字符串。

## 5. 和「过于抽象」的边界

- **不做**：通用 Scene 框架、插件注册、跨引擎 Storybook。
- **做**：`DebugSceneMeta` 加上 `stability` + `mount` 闭包 + 语义 dump；inventory 测；可选 pantry 壳。
- 新组件的定义完成条件 = **至少一条 Evolving Preview 能在真 HostSession 里打开并按键**。要锁再标 Frozen。

这就是「快速迭代同时维持特性」：特性在 Preview 的 invariants 和 Frozen 基线上，不在另一份 design 文档。
