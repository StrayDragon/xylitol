# Design: c2070-add-package-tui-dual-interaction-modes

## 目标边界

| 在范围 | 不在范围（后续 change） |
|---|---|
| 库双模式 seam：Mode A inline ↔ Mode B alt-screen | `c1760` 多级折叠 |
| Mode B：应用内拖选 / 越界续选 / 松手复制 | `c2040`/`c2050` 点折叠三角与段级适配 |
| 输入区（Editor）选区排除 / 抢占规则 | `c1505` viewport slice、`c1535` wrap 优化 |
| 产品 host：模式旗标、默认 Mode A、切换换栈 | 现在把产品默认切到 Mode B |
| 复用已归档 `c2020` mouse 管道 | 回滚 c2020；把 `XYLITOL_TUI_MOUSE` 当产品开关 |

## 模式 oneof（产品语义）

```text
Mode A（emulator-owned）
  inline 差分 + 终端 scrollback + 终端原生选区
  mouse capture 默认关（lab/e2e 可显式开，不承诺点折叠）

Mode B（application-owned）
  alt-buffer（倾向 ?1049h）+ app ScrollView + grabbed mouse
  应用内选区 + 越界 auto-scroll + 松手复制（默认开）
```

未修饰左键在同一会话内只能归属一侧——**禁止**文档暗示 Mode A 开 mouse 后仍保留完整终端选区又同时「自然点折叠」。

## 子系统切分（库内逻辑边界，非文件钉死）

| 子系统 | 职责 | 主要对照 |
|---|---|---|
| **mode / lifecycle** | 进入/退出 Mode B（alt-buffer、mouse enable/teardown）；会话择一；切换换栈 | Pi `TuiMainScreen` / `TuiAltScreen` |
| **viewport / scroll** | 应用视口、滚轮、选区越界续滚 | Pi `ScrollView`；Zellij pane scroll during selection |
| **selection** | anchor/focus、拖选、粒度（字符/词/行 SHOULD）、高亮 | Pi `handleSelectionMouseEvent`；Zellij `Selection` + `mouse_handler` |
| **clipboard** | 松手复制默认开；OSC52 与/或本地工具 | Pi copy；Zellij `copy_on_select` / clipboard |
| **input-exclude** | 底部 Editor/Input 与 chrome 不进 transcript 选区；click 落 input 抢焦点 | Pi AltScreen 输入区特例；Zellij 不可选 pane |
| **hit priority** | 折叠标记 / OSC8 等消费 click 优先于选区起点（折叠实现在后续 change，本 change 留 hook） | Pi starline / link；Zellij unselectable |

`c2020` 已提供：`enable_mouse_capture`、`InputEvent::Mouse`、Moved 不强制刷帧。Mode B 在其之上挂 selection 状态机，**不要**再发明第二套 mouse 扇入。

## 权衡

### 1. alt-buffer 是否强制

| 方案 | 利 | 弊 |
|---|---|---|
| A. 强制 `?1049h`（Pi fullscreen） | 与终端 scrollback 彻底隔离；选区/滚轮语义清晰 | 退出丢主屏历史观感；实现量大 |
| B. 主屏自管视口、不进 alt | 少一条 CSI 路径 | 易与终端 scrollback/选区打架；oneof 更难讲清 |

**倾向 A**（与 proposal Open Q1 产品倾向一致）。Specs 写「Mode B MUST 使用应用自管视口；实现 SHOULD 经 alt-buffer 进入」，允许验证时用等价自管视口，但产品文档以 alt-screen 叙述。

### 2. 复制后端

| 方案 | 利 | 弊 |
|---|---|---|
| OSC52-only | 远程友好、少依赖 | 部分终端禁 OSC52 |
| 本地工具优先 + OSC52 回退 | 对齐 Zellij/crush 实操 | 配置面更大 |

**Specs**：松手复制 MUST 默认开启；后端 MAY OSC52 与/或本地；具体优先级留给 design 实现备注与配置，不在本 change 钉死单一工具名。

### 3. Mode A Alt-hold 点折叠

廉价增强，**不**代替 Mode B，也**不**阻塞本 change。留给产品后续或独立小 change；本 change Out of scope。

### 4. 输入排除：结构 dock vs hit-test

Pi **无**独立 input-exclude API：Editor 在 ScrollView **外**的 dock，选区高亮裁到 ScrollView box；鼠标一律 consume，**不** click-to-focus。Zellij 用 selectable / 子程序 mouse tracking 优先级。

**选定**：Mode B MUST 用「transcript 视口 vs 输入 dock」结构（或等价坐标系排除）；MAY 额外做 click-to-focus（Pi 未做，xylitol 可增强）。折叠 hit 插入「scrollbar/fold > selection」链，实现归后续 change。

## 产品接线

- Host / `TuiRunOptions.interaction_mode`：显式模式选择；**默认 Mode A**。
- 一次会话一个主模式；运行中切换 = teardown 旧栈 + rebuild 根子树 + begin/end Mode B（对齐 Pi 换实现）。
- Mode B：Enable mouse + alt-buffer；每帧 `ModeBRuntime::project_frame` 把 UiRoot 全量行拆成 ScrollView 可见窗 + 下缘 dock；dock 行数优先用 UiRoot 上帧实测（toast+status+editor+footer）。
- teardown MUST Disable / 退缓冲；`with_terminal_suspended` resume 后 MUST 重进 alt + mouse（ptim11）。
- **不**读 `XYLITOL_TUI_MOUSE` 作产品模式开关。

## 验收 seam（自动化优先）

| 锚点 | 建议 |
|---|---|
| 双模式 API / 生命周期 | 包级单测 + VirtualTerminal 记录 CSI/mouse 启停序 |
| 拖选高亮 | VirtualTerminal 或组件级选区模型单测 |
| 越界续选 | 选区状态机单测（拖到顶/底 → scroll 回调） |
| 松手复制 | 注入 clipboard sink；默认开 |
| 输入排除 | 落点在 Editor 矩形 → 不扩展 transcript 选区 / 或取消选区并聚焦 |
| 产品默认 Mode A | host/配置单测或 feature:false 文档场景 |

折叠点击 **不**在本 change 验收。

## 非目标

ratatui 整包替换 · 追平 Pi 全部 chrome · Mode A 承诺无修饰点折叠 · 实现 blocks 内后续 change

## 调研指针

见 `proposal.md`「调研」表；合并结论以 `research/xylitol-mode-b-subsystem-cut.md` 为准。
