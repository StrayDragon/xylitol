# Design: 包级鼠标输入地基（c2020）

## 目标

在差分 inline 引擎上增加**可开关**的鼠标事件通路，使后续 `c2040` 能做点击折叠；本 change **不**实现 fold hit-test / 产品折叠语义。

已拍板：**capture 默认关**（深挖 Q1=A）；显式 API 开启。

## 架构切片

```text
crossterm Event::Mouse
    → map（包 demo start / 产品 map_crossterm_item）
    → InputEvent::Mouse(…)
    → dispatch_event / InputListener / Component::handle_input
    → host：仅 dirty 时 request_render（Moved 默认丢弃或不 dirty）
```

| 层 | 职责 | 非职责 |
|---|---|---|
| `Terminal` / CrosstermTerminal | `enable_mouse_capture` / `disable`；`start` 默认**不** Enable；`stop`/`finish_inline` 若曾 Enable 则 Disable | 解释点击语义 |
| `InputEvent` | 增加 `Mouse` 变体（包装或薄 DTO） | 折叠目标 id |
| demo `start_impl` | 扇入 Mouse；过滤 `Moved` 默认不 `do_render` | 产品 fold |
| 产品 host | `map_crossterm_item` 升 Mouse；`handle_input` 对 Mouse **条件** `request_render` | 默认开 capture |

## 关键决策

| 项 | 决定 | 依据 |
|---|---|---|
| 默认 capture | **关** | Q1=A；crossterm 默认；选区 |
| Enable 序列 | 沿用 crossterm `EnableMouseCapture`（含 1003） | 一手 API；应用层滤 `Moved` |
| `Moved` | 扇入后默认丢弃或标记 not-dirty；**禁止**无条件 render | `diff-engine-mouse-fit.md` |
| Key 路径 | 本波可不改「每次 Input 都 request_render」 | 缩小范围 |
| 与 paste/Kitty | 独立 Command；stop 成对清理 | crossterm 示例 |
| 坐标 API | Mouse 事件带 column/row；viewport→content 映射 **MAY** 本波提供只读 helper，**MUST NOT** 要求产品 fold 表 | 为 c2040 留缝 |

## 权衡

| 方案 | 取舍 |
|---|---|
| A. 默认关 + 显式开（本波） | 安全；点击折叠前产品须调用 enable |
| B. 启动即开 | 发现性好；选区与 Moved 成本高 —— 已否决 |
| 自研 CSI 1000-only（无 1003） | 少 Moved；偏离 crossterm、维护成本高 —— 本波不做 |

## 与差分引擎

- **适合**点击；只能点活视口。
- 主雷在 host 无条件 `request_render`，不在 diff 写屏。
- 详见 `research/diff-engine-mouse-fit.md`。

## Specs 落点（意向）

| Capability | 变更 |
|---|---|
| `package-tui-engine` | `InputEvent::Mouse`；listener/dispatch 可收到；Moved 不强制帧 |
| `package-tui-terminal-protocol` | Enable/Disable mouse；默认关；stop 清理 |
| `app-tui-host` | 扇入 Mouse；Mouse 无 dirty MUST NOT `request_render`（与 ath24 精神对齐到 Input 路径） |

可执行例子：包级 `feature: false` 单测为主（对齐 pte*/tp*）；产品 host 用 harness 计数（ath5 族），必要时薄 `.feature`。

## 非目标

- fold 点击 / leader / 字形（c2030/c2040）
- 滚轮→PageUp、hover 高亮、copy-on-select
- 改 differential 算法 / alt-buffer
