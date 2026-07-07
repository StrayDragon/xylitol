# _HANDOFF — TUI 转向：pi-tui 完整 Rust 重写

> 最后更新：2026-07-07
> 分支：`feat/tui-dev`
> 当前阶段：**自研引擎存档，pi-tui 完整 Rust 实现 + 对接进行中**

---

## 一、决策转向（2026-07-07）

c399 之后的自研 diff 渲染引擎（`src/app/tui/engine/` + `widgets/`，~5600 行）存在多轮对齐 pi 仍未根治的布局 bug。三轮 diff 补丁（`5e11468`/`f635ee2`/`033139d`）+ c400 padding fix 后用户手动验证仍见类似问题。

**决策**：废弃当前自研引擎，改为**基于 pi-tui TypeScript 源码完整重写一个 Rust 版本**（独立 crate），然后对接回 xylitol 的 `Driver`/`XyEvent`/`RenderedLine` seam。

### 为什么完整重写而非 patch

- c399 的对齐方式是「研读 pi 源码 → 选择性移植 Xylitol 需要的部分」→ 漏掉了很多看起来不显眼的细节（padding、viewport 锚定公式、滚动时 CUD 到下缘再 `\r\n`……），每个遗漏都是一轮 debug 循环。
- 完整重写 = 按 pi-tui 文件→Rust 模块逐一移植，结构、命名、行为都保持一致，遗漏概率大幅降低。
- 目标：产出一个可独立测试的 `xylitol-tui-engine` crate（不依赖 xylitol 任何 crate 内类型），然后 xylitol 的 `src/app/tui/` 面作为 consumer 对接。

### 当前自研引擎存档

- c399/c400 的 engine/widgets 代码保留在 `feat/tui-dev` 分支中，作为重写时的**行为参考**（测试用例、宽度工具、`ScrollbackTerminal` oracle 可复用）。
- c400 工件（proposal/design/spec/tasks）保留，记录诊断过程（缺失 pi `Math.max(result.length, termHeight)` padding 的发现）。
- 本 `_HANDOFF.md` 转为记录重写的关键对接契约 + 待复用资产。

---

## 二、关键对接契约（重写不能破坏的边界）

### 必须保留的 seam

| 层 | 模块 | 说明 |
|---|---|---|
| **domain** | `XyEvent` 枚举 | 业务事件词汇，一字不改 |
| **app/core** | `Driver` trait（`run/abort` + `EventStream`） | agent 驱动边界，一字不改 |
| **app/tui** | `RenderedLine` enum + `xyevent_to_rendered` | UI 数据 seam（spec tui42），新引擎只消费 `RenderedLine`，绝不 match `XyEvent` |
| **app/tui** | `commands.rs` 的 `dispatch` + `CommandOutcome` | 斜杠命令分发 |
| **app/tui** | `mod.rs` 的 `Msg` enum + `HostAction` | host loop 消息类型，对接点 |
| **app/tui** | `TuiApp` / `StreamBuffer` | 应用状态 + 流式缓冲 |

### arch_guard 不变量

- TUI 层不 `use crate::agent` / `use crate::infra`
- TUI 层只经 `Driver` + `XyEvent` 与 agent 通信

---

## 三、pi-tui 源码参考（待 1:1 移植的文件清单）

pi 路径：`../pi/packages/tui/src/`

| pi 文件 | 行数 | 职责 | Rust 等价 |
|---|---|---|---|
| `tui.ts` | ~1600 | 核心引擎：doRender/doDiff/scrollback/viewport/overlay | 最核心，优先移植 |
| `terminal.ts` | ~200 | Terminal trait + ProcessTerminal | crossterm 薄封装 |
| `component.ts` | ~300 | Container/Component/Focusable | 已有 `engine/component.rs` 可参考 |
| `utils.ts` | ~500 | visibleWidth/truncate/wrap/sliceByColumn | 已有 `engine/width.rs` 可参考 |
| `input.ts` | ~800 | Input widget + cursor/grapheme/scroll | 已有 `widgets/input.rs` 可参考 |
| `loader.ts` | ~100 | Loader spinner widget | 已有 `widgets/loader.rs` 可参考 |
| `markdown.ts` | ~250 | Markdown widget | 已有 `widgets/markdown.rs` 可参考 |
| `text.ts` | ~100 | Text/TruncatedText | 已有 `widgets/text.rs` 可参考 |
| `box.ts` | ~80 | Box 容器 | 新增 |
| `editor.ts` | ~600 | Editor（多行输入，pi 级） | 新增（xylitol 未来需） |
| `select-list.ts` | ~230 | 列表选择器 | 新增（c356 需要） |
| `settings-list.ts` | ~250 | 设置列表 | 新增 |
| `image.ts` | ~150 | Kitty 图片 | P2 |
| `fuzzy.ts` | ~80 | 模糊匹配 | 工具函数 |
| `stdin-buffer.ts` | ~300 | stdin 缓冲 + OSC reply 拦截 | OSC 11 探测需要 |
| `keys.ts` | ~600 | 键盘三协议解码 | **Rust 不需要**（crossterm 已归一化） |
| **总计** | ~6100 | | |

`keys.ts`（~600 行）是 pi-tui 中唯一可以完全跳过的模块——crossterm 的 `event::read() → KeyEvent` 已归一化 Kitty/modifyOtherKeys/legacy VT 三种协议。

---

## 四、可复用资产（从当前自研引擎搬）

| 资产 | 位置 | 重写时如何用 |
|---|---|---|
| `width.rs`（width/truncate/wrap，CJK-safe） | `src/app/tui/engine/width.rs` | 行为对齐 pi `utils.ts`，可直接搬 |
| `style.rs`（CellStyle/Color/Span/StyledLine + ANSI） | `src/app/tui/engine/style.rs` | 行为对齐 pi 的 SGR 序列化，可直接搬 |
| `ScrollbackTerminal`（scrollback 模拟 test oracle） | `src/app/tui/engine/virtual_terminal.rs` | c400 新增，可直接搬 |
| `VirtualTerminal`（cell-grid test oracle） | `src/app/tui/engine/virtual_terminal.rs` | 可直接搬 |
| `CapturingTerminal`（测试替身） | `src/app/tui/engine/terminal.rs` | 可直接搬 |
| `theme_detect.rs`（COLORFGBG 解析） | `src/app/tui/theme_detect.rs` | 可直接搬 |
| `theme.rs`（Palette） | `src/app/tui/theme.rs` | 适配新 CellStyle 后可用 |
| `syntect_highlight.rs` | `src/app/tui/syntect_highlight.rs` | 依赖于 CellStyle，搬后适配 |

---

## 五、重写后的对接清单

### 删除
- `src/app/tui/engine/tui.rs`（替换为新引擎的 `do_render`）
- `src/app/tui/engine/component.rs`（替换为 pi component.ts 的 Rust 移植）
- `src/app/tui/engine/outcome.rs`（并入新引擎的 UxOutcome）
- `src/app/tui/engine/keybindings.rs`（并入新引擎的 keybindings）
- `src/app/tui/widgets/` 全部（替换为 pi 组件的 Rust 移植）

### 适配
- `src/app/tui/mod.rs`：host loop + 组件树组装，改为用新引擎的 `Tui`/`Container`/`Component`
- `src/app/tui/render.rs`：`xyevent_to_rendered` seam 保留，`to_lines` 适配新 StyledLine
- `src/app/tui/app.rs`：`StreamBuffer`/`TuiApp` 保留，`pending_tail_rows` 适配

### 新增
- `xylitol-tui-engine/` crate（独立 workspace member）
- `src/app/tui/` 原有对接代码适配

---

## 六、历史修复记录（自研引擎期，已存档于分支）

三轮 bug 修复 + c400 padding fix 的完整记录见本文档末尾「历史 commit 清单」。核心教训：

| 发现 | 来源 |
|---|---|
| `COLORFGBG` 拼写（非 `COLORFGBS`） | 首轮 bug 1 |
| spinner idle 时需关 `spinning` flag | 首轮 bug 2 |
| fits-check 用 `<=` 而非 `<`（cursor 覆盖末字反向方案） | 首轮 bug 3 / 二轮 CJK |
| thinking palette 需 apply 到每 span | 首轮 bug 4 |
| `Tick` 需在 idle 也调 `try_render` | 首轮 bug 5 / 三轮 |
| `XyDone` 绕过节流 `render_now()`（16ms 节流跳过终态帧） | 三轮 core bug |
| `Loader::set_spinning(false)` 重置 `current=0` | 三轮 |
| diff 滚动前 CUD 到底行再 `\r\n` | `5e11468` |
| `finalCursor` 跟踪真实落点（不直接设 `last`） | `033139d` |
| all-deletions 分支 + shrink fullRender 回退 | `033139d` |
| 缺少 pi `Math.max(result.length, termHeight)` padding | c400 诊断 |

---

## 七、commit 历史

```
c425c68 update _HANDOFF and agents skills
971de64 refactor(tui): 环境变量 PI_CLEAR_ON_SHRINK → XYLITOL_TUI_CLEAR_ON_SHRINK
033139d fix(tui): 彻底重写 diff 管线对齐 pi —— 修多行流式重叠/重复残存 bug
f635ee2 fix(tui): cursor 回到 reverse 反色块 + 防泄漏（对齐 pi/kimi）
5e11468 fix(tui): 修复超一屏输出重叠/重复 —— 对齐 pi Step 5C scrollback 处理
f1cc2b3 fix(tui): cursor 改纯 fg 不发 bg —— 修复光标 bg 泄漏到后续 span
e3cb290 feat(tui): 输入区光标主题感知 + COLORFGBG 检测接通 Palette
1871c0f docs(sdd): 归档 c399-tui-rewrite-pi-render-engine
03c0b73 fix(tui): c399 loader 状态机 bug
272eead fix(tui): c399 CJK 输入修复
3d47a66 fix(tui): c399 阶段 4 手动验证 bug 修复
3d23965 feat(tui): c399 阶段 4 — 接入新引擎 + 删 ratatui
57d2e2d feat(tui): c399 阶段 3 — UX 层
320332a feat(tui): c399 阶段 2.4 — Loader widget
1529f95 feat(tui): c399 阶段 2.3 — Input widget
7939c44 feat(tui): c399 阶段 2.1+2.2 — 基础 widget + Markdown
4ac31fb feat(tui): c399 阶段 1.6 — virtual_terminal 测试 harness
2cdbd89 docs(sdd): propose c399 TUI pi-render-engine rewrite
```
