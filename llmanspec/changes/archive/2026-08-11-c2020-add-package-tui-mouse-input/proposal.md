---
depends_on: []
blocks:
- c2040-add-tui-mouse-click-fold-triangle
- c2050-update-activity-fold-mouse-leader
branch: sdd/c2020-add-package-tui-mouse-input
base_sha: 04f1894aabe743e08c0623e788ba55a5840755bb
checkpointed: true
checkpoint_sha: 04f1894aabe743e08c0623e788ba55a5840755bb
---

# 包级 TUI 鼠标输入地基（EnableMouse + InputEvent::Mouse）

> **一句话**：在 `xylitol-tui` 打通 crossterm 鼠标事件到 `InputEvent`，可开关，默认策略不毁终端选区；不实现产品折叠点击。

## Why

今日 `InputEvent` 只有 `Key | Paste`；demo/`start` 与产品 host 扇入都丢弃 `Event::Mouse`。`c1760` 已拍「本波不做鼠标、只预留行距缝」；景观审计也标鼠标/选区为引擎缺口。没有包级地基，产品无法做「点三角折叠」、Web 同源动作在 TUI 也缺点击触发面。

本草案只交付**引擎能力**：enable/disable、事件解码、host/demo 扇入、可选 hit 坐标约定。产品语义留给后续 change。

## What Changes

1. **`InputEvent::Mouse(...)`**：包装 crossterm `MouseEvent`（或薄 `Xy*` 友好 DTO）；`Component::handle_input` / `dispatch_event` / listeners 可收到。
2. **终端生命周期**：`Terminal::start/stop`（或显式 API）支持 `EnableMouseCapture` / `DisableMouseCapture`；可配置默认关或「仅产品显式开」。
3. **扇入**：`TUI::start*` 与产品 host 事件环把 `Event::Mouse` 转成 `InputEvent::Mouse`（今日 `_ => {}` 丢掉）。
4. **策略钩（意向）**：包提供 Enable/Disable 开关与文档；**默认关**（对齐 crossterm 默认）。选区 tradeoff 属终端行为，**不是** crossterm 契约——不可写「库保证 Shift 透传选区」；产品侧用关捕获 / 终端习惯缓解。
5. **Move 洪水**：crossterm `EnableMouseCapture` 固定开 `1003` any-event → 会有 `Moved`；应用层 MUST 过滤，未命中可折叠区 MUST NOT dirty/render。
6. **差分引擎接线闸（硬）**：产品 `handle_input` 今日对每次 Input 都 `request_render`——Mouse 路径 MUST 仅在态变 / listener dirty 时请求帧；包 demo 环同理。见 [`research/diff-engine-mouse-fit.md`](./research/diff-engine-mouse-fit.md)。
7. **非目标**：产品 fold hit-test、选区复制、拖拽滚动、Web 面。

## Capabilities（意向）

- `package-tui-input`（或新 `package-tui-mouse`）— 事件与生命周期
- 产品 `app-tui-host` MAY 仅接线扇入；**MUST NOT** 在本 change 实现折叠点击

## 验证（自动化 + 人类）

| 层 | 自动化 | 人类 |
|---|---|---|
| 包 | 合成 Mouse：`Moved` ×N 后 `frame_count`/render 计数不增；`Down` 经 listener 可 `Consumed`；Enable/Disable 成对 | — |
| 产品 host | 注入 `Mouse(Moved)` → **不** bump paint；Key 路径回归仍可刷一帧 | — |
| PTY e2e（可选 `#[ignore]`） | 启停后 mouse mode 不残留（若有探针） | Kitty：开 Enable 后乱晃鼠标无明显空转；关捕获后拖选可用 |
| 回归闸 | `just test-tui` + 产品相关测；`just qa` 不强制 PTY | verify 笔记勾选人类清单 |

**人类最短路径**：显式开 mouse → 晃鼠标无闪烁/空转 → 退出后 shell 选区正常。

## Impact

| 层 | 影响 |
|---|---|
| `packages/xylitol-tui` | `InputEvent`、terminal start/stop、demo 环 |
| `src/app/tui` host | 扇入 `Mouse`；默认可不消费 |
| 性能 | 鼠标事件频率高于按键；未订阅方 MUST 廉价忽略；禁止每 move 整屏 invalidate |

## 依赖与排序

```text
[本 change c2020]  ← 无前置（Wave 0）
       │
       ├─blocks→ c2040（点击三角）
       └─blocks→ c2050（多级适配；亦依赖 c2030/c1760/c2040）
c2030（leader）与本 change 无边，可并行深挖/落地
c1760（多级折叠 MVP）与本 change 无边
```

- **无硬前置**；`blocks`：`c2040`、`c2050`。
- 与 `c1760` / `c2030` 正交（无 `depends_on` 边）。
- 性能并列候补：`c1505` / `c1370` / `c1535`——见 `research/mouse-perf-and-selection.md`。

## Out of scope

- 点三角 / leader 数字 / 多级折叠语义
- copy-on-select / OSC52 选区
- 改 `ScrollbackFold` 全局 bool

## Ethics

- risk_level: medium（开启 mouse capture 会破坏部分终端原生选区）
- prohibited_actions: 默认强制抢选区且无透传；在包内偷渡产品 fold 语义
- required_evidence: 开/关 capture 的终端行为手测（Kitty/常见 tmux）；未消费 Mouse 时无额外整帧重绘
- escalation_policy: 默认改为「始终 EnableMouse」须用户确认

## Further Notes

- 调研：[`research/mouse-perf-and-selection.md`](./research/mouse-perf-and-selection.md)；一手 API：[`research/crossterm-mouse-api.md`](./research/crossterm-mouse-api.md)；**差分×鼠标**：[`research/diff-engine-mouse-fit.md`](./research/diff-engine-mouse-fit.md)
- **差分结论**：适合点击；主雷是 `Moved`×无条件 `request_render`；只能点活视口，不能点已进模拟器 scrollback 的历史
- **crossterm 摘要**：默认不捕获；Enable 含 `1003`；选区副作用库内未文档化
- 相关：景观 §5；`c1760` 深挖 B；`c2040` 消费本地基

## Open Questions

1. ~~Mouse capture 默认~~ → **已拍：A** — 包/产品默认关；显式 API 开启（对齐 crossterm；避免 `1003`/`Moved` 与选区副作用）。`c2040` 再定产品何时调用 enable。
