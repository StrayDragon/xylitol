---
depends_on:
  - c2020-add-package-tui-mouse-input
blocks:
  - c1505-add-tui-scrollback-viewport-slice
  - c1535-optimize-tui-stream-wrap-tail
  - c1760-add-tui-activity-fold
  - c2040-add-tui-mouse-click-fold-triangle
  - c2050-update-activity-fold-mouse-leader
apply_band: P9-deferred
summary: "xylitol-tui 双交互架构（inline ↔ alt-screen）；Mode B 默认含应用选区/跨页选/松手复制；折叠族总前置"
---

# xylitol-tui：双交互架构（终端选区 ↔ 应用内选区）

> **⚠️ deferred（2026-08-11）**：整族在 `llmanspec/delayed-changes/tui/c2070-…/`。
> **角色**：`packages/xylitol-tui` **顶层基础 change**——先构筑双模式，再谈折叠/点击/viewport。
> **目录**：本 `proposal.md` + `research/` + **`cascade/`**（级联后续提案，便于整包移动/升格）。
> **本草案不做** Specs landing / apply，直至调研实现并产品确认升格。

> **一句话**：库支持 **Mode A（inline / emulator-owned）** 与 **Mode B（alt-screen / application-owned）**；产品近期继续打磨 inline；日后可切 alt-screen。Mode B **默认 MUST** 提供不亚于今日 inline 终端选区的：拖选、跨页/越界续选、松手自动复制等。

## Why

调研（见 `research/`）表明未修饰左键归属是 **oneof**；starline 式直接点折叠挂在 Pi fullscreen（AltScreen）一侧。把双架构升为引擎总前置，避免在 inline 上硬塞「自然点折叠」。

### 与已归档 c2020 的关系（必读）

`c2020` 已落地 **opt-in mouse 管道**（包 API + `InputEvent::Mouse` + paint-safe reaction），**应保留**作 Mode B 地基，**不要回滚**。

| | 说明 |
|---|---|
| 保留 | `Terminal::enable_mouse_capture`、事件扇入、Moved 不刷帧、teardown Disable |
| **不是**产品开关 | `XYLITOL_TUI_MOUSE` = **lab / e2e**（`agent_demo`）；产品 `TerminalGuard` **不读**该 env |
| 本 change 补齐 | Mode B（alt-screen）上正式 Enable + 应用内选区 MUST；再挂 `cascade/` 点击折叠 |

详见 [`README.md`](./README.md)「已落地地基：c2020」。

## 产品时序（已拍）

| 阶段 | 做什么 |
|---|---|
| **现在** | 保持本 change **delay**；**继续打磨 inline（Mode A）TUI app** |
| **之后** | 调研并实现本 change（双模式引擎） |
| **再后** | 产品 TUI **可能**切到 Mode B（alt-screen 那一套）；`cascade/` 内折叠/点击等按 `depends_on` 升格落地 |

## What Changes（意向）

1. **双模式 seam（`packages/xylitol-tui`）**
   - **Mode A**：今日路径——inline 差分、终端 scrollback、**终端原生选区**；mouse 默认关或仅瞬时/模式。
   - **Mode B**：**alt-screen（或等价自管视口）** + grabbed mouse + **应用内选区** + app scroll。
2. **Mode B 默认能力 MUST**（对齐「以前 inline 靠终端就能做的事」，参照 Pi `TuiAltScreen`）
   - **拖选**：未修饰左键拖出字符流选区并高亮。
   - **跨页 / 越界续选**：选区拖到视口顶/底时 **自动滚 transcript**，选区可跨出当前屏。
   - **松手自动复制**：button up 后写入剪贴板（OSC52 与/或本地工具），行为可配置但**默认开**。
   - （SHOULD）双击词 / 三击行；与折叠 hit 共存时：点折叠标记消费 click，其余走选区。
3. **产品闸**：设置/旗标择模式；一次会话一个主模式；切换换栈。默认产品面仍 Mode A，直至显式切 B。
4. **非本 change**：折叠三角、L2/L3、per-block 覆盖 → `cascade/c2040` / `c1760` / `c2050`。

## Capabilities（意向）

- `package-tui-*`（双模式 / selection / viewport / clipboard）
- `app-tui-host`（模式选择与生命周期）

## Impact

| 层 | 影响 |
|---|---|
| `xylitol-tui` | Mode B ≈ 新选区+scroll 子系统 |
| 产品 TUI | 近期 Mode A；日后可切 B |
| `cascade/` | 全部 `depends_on` 本 change（或经本 change 间接） |

## 目录与依赖（frontmatter SSOT）

```text
llmanspec/delayed-changes/tui/c2070-add-package-tui-dual-interaction-modes/
  proposal.md          ← 本文件（总前置）
  research/            ← 架构调研
  cascade/
    c1760-…            depends_on: c1755, c2070
    c2040-…            depends_on: c2020, c2070
    c2050-…            depends_on: c1760, c2020, c2040, c2070
    c1505-…            depends_on: c2070
    c1535-…            depends_on: c2070   # 同波；ROI 低
```

```text
c2020（已归档）
  └─ c2070（本 change）
       ├─ c1760 / c2040 / c2050（折叠·点击）
       └─ c1505 / c1535（长历史性能，软相关）
```

升格时：优先整目录移动，或先升本 change 再按 `depends_on` 从 `cascade/` 拎出。

## Out of scope

- 现在就切换产品默认到 Mode B
- 在 Mode A 承诺「无修饰点折叠且原生选区不变」
- 复活 fold-leader；追平 Pi 全部 chrome

## Open Questions（实现调研时钉）

1. Mode B 是否 **必须** `?1049h` alt-buffer，还是允许自管视口留在主屏？（产品倾向：**alt-screen 那一套**）
2. 复制默认 OSC52-only vs 本地工具优先（对齐 Pi / crush）
3. Mode A 是否保留 Alt-hold 点折叠作廉价增强（不代替 B）

## 调研

| 文档 | 内容 |
|---|---|
| [`research/emulator-vs-app-selection-oneof.md`](./research/emulator-vs-app-selection-oneof.md) | 术语与 oneof |
| [`research/native-selection-vs-click-fold.md`](./research/native-selection-vs-click-fold.md) | 方案表 |
| [`research/alt-hold-capture-and-scrollback-hit.md`](./research/alt-hold-capture-and-scrollback-hit.md) | Alt-hold；历史上滚不可点 |
| [`research/pi-starline-click-expand-vs-rust.md`](./research/pi-starline-click-expand-vs-rust.md) | starline / Rust |
| [`research/pi-dual-tui-modes-and-xylitol-cost.md`](./research/pi-dual-tui-modes-and-xylitol-cost.md) | Pi 证据、代价、ratatui |

## Ethics

- Mode A 文档不得暗示「开了 mouse = 自然选区 + 直接点」。
- Mode B 上线时 MUST 文档写清：选区/滚轮归应用；并验收跨页选与松手复制。
