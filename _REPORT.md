# 分析报告：xylitol-tui ↔ pi-tui

> 日期：2026-07-09
> 范围：`packages/xylitol-tui` vs `../pi/packages/tui`（HANDOFF / AGENTS 所称 pi-tui）
> 规范依据：根 / `src/` / `src/app/` / `src/app/tui/` / `packages/xylitol-tui` 的 `AGENTS.md`，以及 `_HANDOFF.md`、`src/app/tui/DESIGN.md`、c445 archive
> **本文是分析快照，不是 SSOT。** 稳定边界以各层 `AGENTS.md` 为准；进度以 `_HANDOFF.md` 与代码为准。

---

## 0. 一句话结论

`xylitol-tui` 已从「完整移植 pi-tui」收敛为：**差分渲染引擎 + 可组合通用组件 + crossterm 原生输入**；产品壳与视觉在 `src/app/tui/`。包侧（含 c445 Container / OverlayHandle）基本就绪，**下一刀是 c450 App Shell 接线**，不是继续 1:1 port。

---

## 1. 对照源与文档地图

| 角色 | 路径 | 说明 |
|---|---|---|
| 对齐源（行为参考） | `../pi/packages/tui` | npm 包名 `@earendil-works/pi-tui`；**不是** `packages/pi-tui` |
| 目标包 | `packages/xylitol-tui` | Rust 引擎 + 通用组件；**零引用**主 crate |
| 产品面（占位） | `src/app/tui/` | 基于包从零重做；`run()` 仍返回明确错误 |
| 视觉 / UX SSOT | `src/app/tui/DESIGN.md` | 语义 token、layout、复制友好；包只收闭包主题 |
| 进度板（非规范） | `_HANDOFF.md` | c445 archived；下一步 c450 |
| 分层 / seam | `src/AGENTS.md` | TUI 面 🟡；走 `Driver` / `dispatch` / `composition` |

**依赖方向（不得回退）：**

```text
src/app/tui  →  packages/xylitol-tui  （引擎/组件）
src/app/tui  →  app/core/{driver,dispatch,composition}  （agent seam）
packages/xylitol-tui  ↛  xylitol 主 crate
```

缺底层能力：**先改包，再接线应用面**（`src/app/tui/AGENTS.md`）。

---

## 2. 战略定位：刻意非 1:1

| 维度 | pi-tui | xylitol-tui（决议） |
|---|---|---|
| 目标 | TS 通用 TUI + coding-agent 可直接 `start()` | 引擎库；产品事件循环在应用面 |
| 输入 | 自管 VT / `stdin-buffer` / native modifiers | **硬切** `InputEvent::{Key,Paste}` + crossterm |
| 根类型 | `TUI extends Container` | `TUI` 持有根列表 + 独立 `Container` 组件 |
| 图片 | 完整 Kitty/iTerm + `Image` | **裁剪**；保留 `is_image_line` / `hyperlink` |
| Overlay | 完整 focus-restore 状态机 | 最小 hide/focus；完整 restore **延后**（c445 `future.md`） |
| 产品壳 | coding-agent 包内 UI | **不进本包** → `src/app/tui/` |

回退成「完整 port」会被 AGENTS 明确拒绝。

---

## 3. 模块清单对照

### 3.1 核心 / 基础设施

| pi (`src/`) | xylitol (`src/`) | 状态 |
|---|---|---|
| `index.ts` | `lib.rs` | 重塑的 API 边界 |
| `tui.ts` | `tui.rs` + `components/container.rs` | 拆分：组合而非继承 |
| `terminal.ts` | `terminal.rs` | 实现不同：`CrosstermTerminal` |
| `terminal-colors.ts` | `terminal_colors.rs` | 有；TUI 级 OSC11 query 未移植 |
| `terminal-image.ts` | `terminal_image.rs` | **部分**：仅豁免 + hyperlink |
| `keys.ts` / `keybindings.ts` | 同名 `.rs` | 运行时走 `KeyEvent` |
| `utils` / `fuzzy` / `autocomplete` | 同名 + `autocomplete_fd.rs` | xy 多 fd 路径 walk |
| `editor-component` / `kill-ring` / `undo-stack` / `word-navigation` | 同名 | 有 |
| `stdin-buffer.ts` | — | **有意不移植** |
| `native-modifiers.ts` + `native/{darwin,win32}` | — | **有意不移植** |
| — | `clock.rs` | xy 独有（确定性时序） |
| — | `paste_burst.rs` | xy 独有（非 bracketed paste 启发式） |

### 3.2 组件

| pi | xylitol | 状态 |
|---|---|---|
| Text / TruncatedText / Input / Editor | 同名 | 有（Editor 构造不持 `TUI` 引用） |
| Markdown / Loader / CancellableLoader | 同名 | 有 |
| SelectList / SettingsList / Spacer | 同名 | 有 |
| `Box` | `Panel` | **重命名** |
| Container（在 `tui.ts`） | `components/container.rs` | **拆出**；不扇出 input |
| `Image` | — | **c445 删除** |

规模量级：pi ~28 个 `.ts` 源文件；xy ~30 个 `.rs` 源文件（~11.5k LOC，含 `tui.rs` / `utils.rs` 大块）。

---

## 4. 架构差异（有代码事实）

### 4.1 根对象：继承 → 组合

- **pi：** `export class TUI extends Container`
- **xy：** `TUI { components: Vec<Box<dyn Component>> }`；`Container` 是普通垂直栈组件，**不**把 `InputEvent` 扇出到子节点（与 pi Container 一致）

c450 可用 `Container` 组 transcript/status/editor，但焦点必须落在可 `Focusable` 的叶子，或由自定义根组件转发。

### 4.2 事件循环：库内 `start` → host 驱动

| API | 用途 |
|---|---|
| `dispatch_event(InputEvent)` | 注入按键 / paste |
| `request_render` / `try_render` | 调度差分渲染 |
| `idle_tick` | 推进 `Component::tick`（spinner 等） |
| `TUI::start` / `start_with_flag` | **仅 demo**（`agent_demo`） |

异步合流（tokio + `XyEvent`）**禁止**进包，落在 `src/app/tui/`。

### 4.3 输入硬切

```rust
pub enum InputEvent {
    Key(KeyEvent),
    Paste(String),
}
```

禁止运行时 KeyEvent→VT→parse。`matches_key` / `parse_key` 仅测试与配置字符串；运行时用 `matches_key_event` / `KeybindingsManager::matches_event`。

### 4.4 终端 I/O

- **pi：** 自管 raw stdin、`StdinBuffer`、大量手写 VT
- **xy：** 优先 crossterm `Command`；仅 OSC 9;4 等库未暴露的才 `write_raw`

### 4.5 Overlay

- **pi：** `OverlayHandle` + eligible/blocked/resume；`unfocus({ target })`；`addInputListener`
- **xy（c445）：** `OverlayHandle { overlay_id }` 世代句柄；hide 时焦点回 `pre_focus` 或下一 capturing overlay；**完整 restore / InputListener 进 future**

### 4.6 光标

默认 **隐藏硬件光标**；Editor 用反色假光标。有 `CURSOR_MARKER` 时可相对定位 IME，但不得无条件 `show_cursor`（防流式闪烁）。

### 4.7 渲染输出

两边均为 `string[]` / `Vec<String>`（ANSI）。**不**引入结构化 `StyledLine`。

---

## 5. 公共 API 摘要

| 能力 | pi | xylitol | 备注 |
|---|---|---|---|
| Container | 基类 | 独立组件 + re-export | c445 |
| OverlayHandle | 闭包式 | id 世代 + `&mut TUI` 方法 | c445 |
| Image / encode | 完整 | 无 | 可按需 feature-gate 恢复 |
| InputEvent | 无（string） | 有 | 硬切 |
| host API | 弱（靠 start） | `dispatch`/`try_render`/`idle_tick` | 产品路径 |
| getKeybindings / TUI_KEYBINDINGS | 有 | 无（`create_default_definitions`） | |
| StdinBuffer | 有 | 无 | |
| PasteBurst / Clock / DebouncedAutocomplete | 无/弱 | 有 | xy 增强 |
| InputListener | 有 | **未移植** | c450 全局快捷键需应用面处理 |

`lib.rs` re-export = 包 API SSOT。

---

## 6. 测试与验证

### xylitol 五层（`test-tui-harness`）

| 层 | 落点 | 作用 |
|---|---|---|
| 1 按键→状态 | `harness_test` + `TuiTestHarness` | 交互序列 |
| 2 snapshot | `snapshot_test` + insta | 布局/换行回归 |
| 3 时序 | `paste_burst` / debounce / `MockClock` | 禁 `thread::sleep` |
| 4 proptest | `property_test` | Editor 不变量 |
| 5 E2E | `tests/tui_e2e` + `just test-tui-e2e` | PTY/tmux + `agent_demo`（`#[ignore]`） |

日常：`cargo test -p xylitol-tui`（1–4）。HANDOFF：c445 后包测绿。

### 相对 pi

pi 用 vitest + `virtual-terminal`，在 stdin-buffer、完整 image、OSC11 TUI 集成、overlay restore 边角上更密；**无**五层 taxonomy、**无**包内 PTY E2E、**无** `agent_demo` 产品形态锁。

---

## 7. `agent_demo`：产品形态锚点

路径：`packages/xylitol-tui/examples/agent_demo.rs`（≈ `DESIGN.md` 图 2）。

| 已验证 | 说明 |
|---|---|
| 单列栈 | transcript → status(0\|1) → editor\|selector → footer |
| Editor 操作区 | 上下 muted `─` 边框 |
| 槽替换 | Ctrl+P → SelectList；Ctrl+S → SettingsList；Esc 还原 |
| 可展开块 | thinking / tool 折叠摘要（应用面实现，非包组件） |
| Glyph | unicode/ascii 配置档；无字体探测 |
| 光标 | 流式时硬件光标隐藏 |
| 循环 | 使用 `start_with_flag` — **产品路径勿抄** |

`tests/agent_demo_test.rs` 锁上述行为。c450 应**迁移模式**到 `src/app/tui/`，而不是把 demo 升格为产品。

---

## 8. 与 DESIGN.md / 分层的对齐度

| DESIGN / AGENTS 要求 | 包就绪度 | 缺口归属 |
|---|---|---|
| 差分渲染 + scrollback 贴尾 | ✅ | — |
| host 驱动同步引擎 | ✅ API | 应用面未接线 |
| Container 组 layout | ✅ c445 | 焦点路由需应用面设计 |
| Overlay 确认框 | ✅ 最小句柄 | 嵌套 restore 延后 |
| Editor / SelectList / SettingsList / Loader / Markdown | ✅ | theme token 映射在面 |
| 可展开 thinking/tool | ⚠️ demo 有 | 面实现；**策略 c450 后再议** |
| footer / transcript | ❌ 非包组件 | 面用 Text/Markdown/自定义 |
| Driver / XyEvent / slash | ❌（正确） | `app/core` + `protocol::Command` |
| 语义色 / glyph 配置 | ❌（正确） | `DESIGN.md` + 面 theme |
| Image 内联 | ❌ 裁剪 | 非默认路径 |

---

## 9. 当前阶段与风险（对照 HANDOFF）

### 已完成

- 引擎 port + 输入硬切 + 五层测试
- c445：`Container`、`OverlayHandle`、Image 裁剪（archived）
- `agent_demo` 验证目标 UX 骨架

### 进行中 / 下一步

1. **propose + 实施 c450（App Shell）**
   `write-surface`（先 `audit-dead-code`）→ tokio 合流 + host 驱动 → `Container` 组栈 → overlay 用 `OverlayHandle` → UX 对齐 `DESIGN.md`
2. 缺底层能力 → 先改 `packages/xylitol-tui` 再接线
3. （可选）Image encode feature-gate

### c450 最高风险

1. **Host 循环 + `XyEvent` 异步合流** — 包内无示范，只能在应用面做对
2. **`Container` 不路由输入** — 焦点/转发设计错误会导致「按键无响应」
3. **复制 `agent_demo` 成巨石根组件** — 应拆 transcript/status/editor/footer，避免再造 `FakeCodingAgentApp` 单体
4. **无 `InputListener`** — 全局 slash/debug 键需在 `dispatch_event` 前或根组件内拦截
5. **嵌套 overlay** — 若确认框叠在选择器上，最小 restore 可能不够（触发 c445 future）
6. **展开策略未锁** — DESIGN 只要求「可展开」；勿在 c450 过早固化快捷键/记忆策略

---

## 10. Specs / SDD 指针

| 前缀 | 用途 |
|---|---|
| `package-tui-*` | 包能力（engine / testing / paste-burst / editor…） |
| `app-tui` | 产品面（c450 起） |

范例归档：`llmanspec/changes/archive/2026-07-09-c445-add-package-tui-container-overlay/`
（含 `design.md` D1–D5、`future.md` 延后项）

工作流：`/llman-sdd-propose` → 实现 → `just qa` → `/llman-sdd-archive`。

---

## 11. 建议阅读顺序（开工 c450）

1. `_HANDOFF.md` §〇
2. `packages/xylitol-tui/AGENTS.md`（刻意差异表）
3. `src/app/tui/AGENTS.md` + `DESIGN.md`
4. `write-tui` / `write-surface` / `test-tui-harness` skills
5. `examples/agent_demo.rs` + c445 `design.md` / `future.md`
6. 需要行为细节时再对照 `../pi/packages/tui/src/tui.ts`（勿逐文件镜像）

---

## 12. 附录：关键签名对照

```typescript
// pi
export class TUI extends Container {
  setFocus(component: Component | null): void
  showOverlay(component: Component, options?: OverlayOptions): OverlayHandle
  start(): void
  addInputListener(listener: InputListener): () => void
}
```

```rust
// xylitol
pub struct TUI<T: Terminal> { /* components, overlays, … */ }
impl<T: Terminal> TUI<T> {
    pub fn set_focus(&mut self, index: Option<usize>);
    pub fn show_overlay(&mut self, component: Box<dyn Component>, options: OverlayOptions) -> OverlayHandle;
    pub fn dispatch_event(&mut self, event: InputEvent);
    pub fn idle_tick(&mut self) -> bool;
    pub fn try_render(&mut self) -> Result<bool, RenderError>;
    pub fn start(&mut self) -> Result<(), Box<dyn std::error::Error>>; // demo only
}
```

---

*报告结束。若需把某一节展开成 c450 接线清单或 overlay restore 差距表，可在此文件上增量追加，勿把进度抄回 AGENTS.md。*
