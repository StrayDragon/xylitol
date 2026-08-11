---
depends_on:
- c2020-add-package-tui-mouse-input
blocks:
- c1505-add-tui-scrollback-viewport-slice
- c1535-optimize-tui-stream-wrap-tail
- c1760-add-tui-activity-fold
- c2040-add-tui-mouse-click-fold-triangle
- c2050-update-activity-fold-mouse-leader
branch: sdd/c2070-add-package-tui-dual-interaction-modes
base_sha: 1dd5de3a53e28365099c2351abd0a767bbfab688
checkpointed: false
---

# xylitol-tui：双交互架构（终端选区 ↔ 应用内选区）

> **升格（2026-08-11）**：自 `delayed-changes/tui/` 整包升入 `llmanspec/changes/`。本 change 是**完整大需求**（库双模式 + Mode B 选区 MUST）；级联后续已拆为独立 change（见 `blocks`），各自带 `depends_on`。
> **角色**：`packages/xylitol-tui` 顶层基础——先构筑双模式，再谈折叠/点击/viewport。
> **FF 状态**：规划壳 + 调研准备 → Branch binding → Specs landing；产品默认仍 Mode A，直至显式切 B。

> **一句话**：库支持 **Mode A（inline / emulator-owned）** 与 **Mode B（alt-screen / application-owned）**；产品近期继续打磨 inline；日后可切 alt-screen。Mode B **默认 MUST** 提供不亚于今日 inline 终端选区的：拖选、跨页/越界续选、松手自动复制等。

## Why

调研（见 `research/`）表明未修饰左键归属是 **oneof**；starline 式直接点折叠挂在 Pi fullscreen（AltScreen）一侧。把双架构升为引擎总前置，避免在 inline 上硬塞「自然点折叠」。

### 与已归档 c2020 的关系（必读）

`c2020` 已落地 **opt-in mouse 管道**（包 API + `InputEvent::Mouse` + paint-safe reaction），**应保留**作 Mode B 地基，**不要回滚**。

| | 说明 |
|---|---|
| 保留 | `Terminal::enable_mouse_capture`、事件扇入、Moved 不刷帧、teardown Disable |
| **不是**产品开关 | `XYLITOL_TUI_MOUSE` = **lab / e2e**（`agent_demo`）；产品 `TerminalGuard` **不读**该 env |
| 本 change 补齐 | Mode B（alt-screen）上正式 Enable + 应用内选区 MUST；再由独立后续 change 挂点击折叠 |

详见 [`README.md`](./README.md)「已落地地基：c2020」。

## 产品时序（已拍）

| 阶段 | 做什么 |
|---|---|
| **本 change** | 升格 + 调研 + Specs + **完整 Mode B 实现**（库视口/选区/OSC52 + 产品换栈/dock）；产品**默认仍 Mode A**，经 `TuiRunOptions.interaction_mode` 显式切 B |
| **之后** | `blocks` 内折叠/点击/性能 change 按 `depends_on` 各自落地；产品是否默认切 B 另议 |

## What Changes

1. **双模式 seam（`packages/xylitol-tui`）**
   - **Mode A**：今日路径——inline 差分、终端 scrollback、**终端原生选区**；mouse 默认关或仅瞬时/模式。
   - **Mode B**：**alt-screen（或等价自管视口）** + grabbed mouse + **应用内选区** + app scroll。
2. **Mode B 默认能力 MUST**（对齐「以前 inline 靠终端就能做的事」，参照 Pi `TuiAltScreen` + Zellij 选区滚动/复制）
   - **拖选**：未修饰左键拖出字符流选区并高亮。
   - **跨页 / 越界续选**：选区拖到视口顶/底时 **自动滚 transcript**，选区可跨出当前屏。
   - **松手自动复制**：button up 后写入剪贴板（OSC52 与/或本地工具），行为可配置但**默认开**。
   - （SHOULD）双击词 / 三击行；与折叠 hit 共存时：点折叠标记消费 click，其余走选区。
   - **输入区特例**：底部 Editor/Input 行 MUST 可排除或短路选区（借鉴 Pi / Zellij 不可选区域处理）。
3. **产品闸**：设置/旗标择模式；一次会话一个主模式；切换换栈。默认产品面仍 Mode A，直至显式切 B。
4. **非本 change**：折叠三角、L2/L3、per-block 覆盖、viewport slice、stream wrap → 见 `blocks` 所列独立 change。

## Capabilities

- `package-tui-interaction-modes`（新建：双模式 / Mode B 选区·滚动·复制·输入排除）
- `package-tui-terminal-protocol` / `package-tui-engine`（衔接既有 mouse / alt-buffer 协议面，按需增量）
- `app-tui-host`（模式选择与生命周期；默认 Mode A）

## Impact

| 层 | 影响 |
|---|---|
| `xylitol-tui` | Mode B ≈ 新选区+scroll+clipboard 子系统 |
| 产品 TUI | 近期 Mode A；日后可切 B |
| 后续 change | `blocks` 全部 `depends_on` 本 change（或经本 change 间接） |

## 依赖图（frontmatter SSOT）

```text
c2020（已归档）
  └─ c2070（本 change · 完整大需求）
       ├─ c1760 / c2040 / c2050（折叠·点击）
       └─ c1505 / c1535（长历史性能，软相关）
```

## Out of scope

- 现在就切换产品默认到 Mode B
- 在 Mode A 承诺「无修饰点折叠且原生选区不变」
- 复活 fold-leader；追平 Pi 全部 chrome
- 实现 `blocks` 内后续 change

## Open Questions（调研后钉）

1. Mode B 是否 **必须** `?1049h` alt-buffer，还是允许自管视口留在主屏？
   **钉**：产品叙述与实现 **倾向 alt-screen**；合约写「应用自管视口 MUST + alt-buffer SHOULD」（见 `ptim02`）。
2. 复制默认 OSC52-only vs 本地工具优先？
   **钉**：默认松手复制 **开**；后端允许 OSC52 与/或本地（Pi AltScreen = OSC52-only；Zellij = `copy_command` 否则 OSC52）。不在合约钉死单一工具名。
3. Mode A 是否保留 Alt-hold 点折叠作廉价增强（不代替 B）？
   **推迟**：不阻塞本 change；非本 change 交付。

## 调研

| 文档 | 内容 |
|---|---|
| [`research/emulator-vs-app-selection-oneof.md`](./research/emulator-vs-app-selection-oneof.md) | 术语与 oneof |
| [`research/native-selection-vs-click-fold.md`](./research/native-selection-vs-click-fold.md) | 方案表 |
| [`research/alt-hold-capture-and-scrollback-hit.md`](./research/alt-hold-capture-and-scrollback-hit.md) | Alt-hold；历史上滚不可点 |
| [`research/pi-starline-click-expand-vs-rust.md`](./research/pi-starline-click-expand-vs-rust.md) | starline / Rust |
| [`research/pi-dual-tui-modes-and-xylitol-cost.md`](./research/pi-dual-tui-modes-and-xylitol-cost.md) | Pi 证据、代价、ratatui |
| [`research/pi-altscreen-selection-scroll-copy-input.md`](./research/pi-altscreen-selection-scroll-copy-input.md) | Pi AltScreen 选区/滚动/复制/输入 |
| [`research/zellij-selection-scroll-copy-input.md`](./research/zellij-selection-scroll-copy-input.md) | Zellij 选区滚动/复制/特例 |
| [`research/xylitol-mode-b-subsystem-cut.md`](./research/xylitol-mode-b-subsystem-cut.md) | xylitol Mode B 子系统切分建议 |

## Further Notes

- 升格决策：cascade 五件拆为 `llmanspec/changes/<id>/` 独立草案，**不**再嵌套 `cascade/`；依赖只以 YAML `depends_on`/`blocks` 为 SSOT。
- 一手对照：Pi `packages/tui`（`TuiMainScreen` / `TuiAltScreen`）+ Zellij `panes/selection.rs` / `tab/mouse_handler.rs` / clipboard；结论摘要见 `research/xylitol-mode-b-subsystem-cut.md` 与专篇。
- Specs landing：`package-tui-interaction-modes`（`ptim01`–`ptim11`）；`app-tui-host` `ath30`（默认 Mode A + Mode B dock/换栈）。
- **完整 Mode B 交付**：`ModeBRuntime`（ScrollView + Selection + dock 投影）挂在 `TUI` 应用会话上；产品 host 换栈并登记实测 dock 行。

## Ethics

- Mode A 文档不得暗示「开了 mouse = 自然选区 + 直接点」。
- Mode B 上线时 MUST 文档写清：选区/滚轮归应用；并验收跨页选与松手复制。
