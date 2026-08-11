# Research: 差分 inline 引擎 × 鼠标交互适切性与性能

> 跨 `c2020`–`c2050`；落点在 c2020（地基约束）。
> 一手：本仓 `packages/xylitol-tui` + `src/app/tui/host`；技能 `terminal-tui-differential-rendering`。
> 结论先读 §0。

## 0. 结论（可引用）

| 问题 | 答案 |
|---|---|
| 差分 / inline TUI **适不适合**鼠标？ | **适合点击类交互**（折叠三角、列表项），**不适合**把终端当完整 GUI（拖选、悬停高亮、点历史 scrollback） |
| 有没有性能雷？ | **有，且在接线层不在 diff 算法**：① crossterm Enable 开 `1003` → `Moved` 洪水；② 产品 `HostSession::handle_input` **每次 Input 都 `request_render`**——若不过滤，鼠标会逼出 ~60fps 全树 `render()` |
| 要不要先换 alt-buffer？ | **不必**。点击折叠不要求 alt-buffer；换缓冲是另一条产品/引擎 change |

**着手门槛（c2020 MUST）**：Mouse 扇入路径上，`Moved`（及无态变的 Down 之外事件）**不得**无条件 `request_render`；仅状态变更或显式 dirty 才请求帧。

## 1. 引擎模式事实

- xylitol-tui = **inline 差分**：`previous_lines` vs `new_lines`，只写变更行 + CSI 2026 同步输出；内容过长滚进**模拟器 scrollback**（见 `previous_viewport_top`）。
- 产品路径：host `dispatch_event` → `request_render` / `try_render`（16ms 节流）；`TUI::start` 仅 demo。
- 输入合约今日：`InputEvent::{Key,Paste}`；host `map_crossterm_item` 丢弃非 Key/Paste/Resize（含 Mouse）。

证据：`packages/xylitol-tui/src/tui.rs`（`do_render` / `differential_render` / `try_render`）；`src/app/tui/mod.rs` `map_crossterm_item`；`src/app/tui/host/mod.rs` `handle_input` 末尾恒 `request_render(false)`。

## 2. 适切性：坐标与命中

差分引擎是 **行数组**模型，不是 retained widget tree with absolute boxes。

点击可行条件：

1. 鼠标 `(column, row)` = **当前终端可见单元格**（crossterm 约定）。
2. 内容行索引意向：`content_line ≈ previous_viewport_top + screen_row`（与引擎把 hardware cursor 映射到 content 的同一套 viewport 算术；实现时对照 `differential_render` 内 `screen_row` 计算，单测钉死）。
3. 产品再把 `content_line` → scrollback entry / fold hit 表（`c2040`/`c2050`）。

**固有限制（产品须诚实）**：

| 限制 | 含义 |
|---|---|
| 只能点「活视口」 | 已滚进终端 scrollback、不在差分 viewport 内的历史行 → 应用收不到有意义 hit（那是模拟器缓冲区） |
| 无悬停层 | `Moved` 若用来做 hover 高亮 = 每动一格可能改一行样式 → 与差分友好，但易刷；**首波折叠点击不做 hover** |
| 单焦点键路由 | 鼠标不应抢 Editor 常驻焦点；host / `InputListener` 消费点击（对齐 c1760 深挖 B） |
| 选区 | Enable capture 后原生拖选变难（crossterm 未保证 Shift 透传；见 `crossterm-mouse-api.md`） |

→ **点击折叠 / leader 编号高亮**与差分模式兼容；**不要**承诺「点任意历史行」「拖选复制与 capture 同时完美」。

## 3. 性能：雷点与闸

### 3.1 Moved 洪水 × 无条件 request_render（主雷）

```text
EnableMouseCapture → CSI ?1003h（any-event）
  → 大量 Event::Mouse { Moved }
  → 若 map 成 HostEvent::Input
  → handle_input 末尾 request_render(false)   // 今日对 Key 亦如此
  → try_render ~16ms 一帧仍跑满组件 render() + diff
```

- **写屏**：行未变时 differential 几乎不写——还好。
- **CPU**：`comp.render(width)`（产品含 scrollback / Markdown / cache 逻辑）仍会跑；长会话下与 ath25 / c1505 压力同族。
- **Tick 闸（ath24）保不住这条路径**：ath24 管的是 idle Tick；Input 路径今日无「无 dirty 则跳过」。

**缓解（合约级，写入 c2020/c2040）**：

1. 扇入：默认丢弃 `Moved`（及可选 `Drag`）；只升 `Down`/`Up`（或合成 Click）。
2. Host：`handle_input` 对 Mouse **仅当 handler 返回 dirty / Consumed 且态变** 才 `request_render`；Key 路径可暂保持现状。
3. 包 demo `start_impl`：同样过滤，避免 demo 环每 move `do_render`。

### 3.2 点击后真态变（可接受）

Fold toggle → 行数变化 → differential shrink/grow 路径已存在；与键盘 Alt+E 同类。
Paint-cache：只失效目标 entry（ath25）；**禁止**因 Mouse 无命中而 `request_render(true)` 清屏。

### 3.3 与 c1370 / c1505

- 鼠标不替代热缓冲封顶 / viewport 切片。
- 少画（c1760 L2/L3）降低每帧 `render` 成本，使误触发 render 的伤害变小，但 **不能**当 Moved 过滤的替代。

## 4. 对 SDD 各 change 的约束回写

| Change | 约束 |
|---|---|
| **c2020** | API + 扇入 + **Moved 默认丢弃** + 「Mouse 无态变不 request_render」钩；验证用 render 计数 |
| **c2030** | Leader 高亮 = 有限行样式变更；进入/退出各一帧；禁止每 digit 全历史 MD 重解析 |
| **c2040** | Hit 表与 viewport 同代；click→toggle 一帧；错点无 render |
| **c2050** | 段/块 hit 仍 O(可见)；矩阵测混合 L1+L2 |

## 5. 验证分层（本波通用模板）

| 层 | 做什么 | 谁跑 |
|---|---|---|
| 包单测 / harness | 合成 `InputEvent::Mouse`；Moved 不增 `frame_count` / render 计数；Down 可被 listener 消费 | `just test-tui` / 产品 harness |
| 产品 harness | 无真 TTY：注入 Mouse → 单块 fold 态变 + paint miss 上界 | `src/app/tui` tests |
| PTY e2e（可选） | Enable/Disable 成对；退出无残留 mouse mode（探针 CSI） | `just test-tui-e2e-pty`，`#[ignore]` 默认可 |
| 人类 | Kitty + 常见 tmux：开 capture 后点标记 / 关 capture 后拖选；流式时乱晃鼠标 CPU 不明显升 | 清单见各 proposal「人类验证」 |

## 6. 不在本调研范围

- 是否默认 Enable（深挖 Q1）
- 滚轮是否映射 PageUp/Down（另案）
- Web 点击（DOM，非差分引擎）
