# Tasks: c2020-add-package-tui-mouse-input

垂直切片；每项可独立验证。Seam：**S1–S5 必做 + S6 PTY 强制**（propose 已确认）。

## 1. Specs landing + 包 Mouse 事件通路（S1）

- [x] 1.1 在绑定分支编辑 live `package-tui-engine`：`InputEvent` MUST 含 Mouse；`dispatch_event` / `add_input_listener` MUST 可接收；文档场景或单测 `feature: false` 对齐 pte 风格
- [x] 1.2 实现 `InputEvent::Mouse`（包装 crossterm `MouseEvent` 或薄 DTO）；更新 `Component` 匹配臂与既有测试编译
- [x] 1.3 包单测：合成 Mouse → listener `Consumed` 阻止焦点组件；未消费则到达 focused child（S1）

## 2. Terminal capture 生命周期（S2）

- [x] 2.1 live `package-tui-terminal-protocol`：默认 MUST NOT Enable；显式 enable/disable；`stop`/`finish_inline` 若曾 Enable 则 MUST Disable
- [x] 2.2 `CrosstermTerminal`（及 `Terminal` trait 若需要）暴露 enable/disable；`start` 默认不 Enable（Q1=A）
- [x] 2.3 VirtualTerminal / 单测：记录 enable/disable 调用序；stop 成对（S2）

## 3. Demo 扇入 + Moved 不刷帧（S3）

- [x] 3.1 `TUI::start_impl`：扇入 `Event::Mouse` → `InputEvent::Mouse`；默认丢弃或忽略 `Moved`（及无态变路径）**MUST NOT** `do_render`
- [x] 3.2 包测：连续合成 Moved 后 `frame_count` 不增；Down 经 listener dirty 路径可增（S3）

## 4. 产品 host 扇入 + 条件 render（S4/S5）

- [x] 4.1 live `app-tui-host`：Ready 扇入 Mouse；Mouse 无 dirty MUST NOT `request_render`；Key 路径本波保持既有「每次可刷」除非显式收窄
- [x] 4.2 `map_crossterm_item` 映射 `Event::Mouse` → `HostEvent::Input(Mouse)`（可先过滤 Moved 不升事件）
- [x] 4.3 `handle_input`：Mouse 仅 dirty/Consumed-且态变时 `request_render`；harness：Moved×N 不 bump paint；Key 回归仍可刷（S4/S5）

## 5. PTY e2e 强制（S6）

- [x] 5.1 增加（或扩展）`#[ignore]` PTY 用例：产品或 `agent_demo` 路径显式 Enable → 收事件或探针 → Disable/退出后 mouse mode 不残留（尽探针所能）
- [x] 5.2 接线：`just test-tui-e2e-pty`（或现有 e2e recipe）可拉取该用例；**默认 `just qa` MUST NOT 强制**（对齐既有 ignore 纪律）
- [x] 5.3 人类清单写入 change `research/` 或 verify 笔记：Kitty 开 Enable 晃鼠标无空转；关捕获后拖选可用

## 6. 闸与文档

- [x] 6.1 `just test-tui` + 相关产品测绿；`llman sdd validate c2020… --strict`
- [x] 6.2 更新包 `AGENTS.md` / `PI_DELTAS` 仅当输入模型差异需台账（Mouse 变体一行即可）；禁止写进度板
