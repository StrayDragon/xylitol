# Design — TUI 流式渲染预算与合帧降频

## 目标

把高 token 速率下的 TUI CPU 从单核 85–100% 降到 <20%（release，1000 delta/s），不牺牲首字延迟与状态可见性；不改产品语义。

## 实测（tmux + 本地 Responses SSE 假模型）

| 场景 | debug | release |
|---|---|---|
| 25–250 delta/s | ~1% | 未测 |
| 1000 delta/s | 5.53s CPU / 5.54s（~100%） | 1.66s / 1.97s（~85%） |
| tmux server | ~1% | ~1–3% |

渲染 lab（`lab_ao_stream_delta_perf_report`）：

- 160 delta 逐 token：160 帧，Σ do_render 186,092µs，851 帧/s。
- 按 8 delta 合帧：20 帧，Σ 22,833µs；渲染时间 8.15× 下降。

## 现状路径

1. runtime 每个 chunk：`TextDelta` → `MessageUpdate`（`crates/xylitol-runtime/src/agent/runtime/react/mod.rs:1375-1381`）。
2. TUI 合批：`is_coalesceable_stream_delta` 只认 `TextDelta`/`ThinkingDelta`（`crates/xylitol-app-tui/src/lib.rs:644`）；`MessageUpdate` 打断。
3. 每个 `MessageUpdate`：`apply_xy_unfenced` → `sync_ui_root_from_model`（`host/mod.rs:845`）→ 全量 clone `UiModel`（`layout/root/mod.rs:1214`）→ bump upper gen（`layout/root/render.rs:323`）→ 重建 scrollback（`widgets/scrollback/mod.rs:213`）→ 指纹（`cache.rs:51`）→ `step_paint_only`。
4. `mark_ao_components_stale` 每次 delta（`host/mod.rs:854`）→ `try_render` 的 16ms 节流条件被绕过（`crates/xylitol-tui/src/tui/render.rs:59`）。
5. busy tick 16ms（`lib.rs:289`）；流式期间恒开。
6. 每帧新建 Markdown 解析器（`widgets/scrollback/paint.rs:266`）；引擎 Markdown 实例缓存未命中（`crates/xylitol-tui/src/components/markdown.rs:195`）。
7. 指纹哈希完整 tool output / write_content（`cache.rs:51`）。
8. AO `set_lines` 全量 clone（`crates/xylitol-tui/src/application_owned_runtime.rs:195`）。

## 方案

### 1. 合批（优先）

- `is_coalesceable_stream_delta` 增加 `MessageUpdate`；合批循环取最后一个 `MessageUpdate`（累积快照），中间的直接丢弃。
- 窗口 30–50ms（默认 40ms）；首帧仍立即（首字延迟不退化）。
- `ThinkingDelta` / `TextDelta` 仍按增量累加；`MessageUpdate` 只用于 tool intent / partial args 同步。
- 合批期间不调用 `step_paint_only`；窗口到期或事件边界（ToolStart/End、AgentEnd、Input）强制 flush。
- 预计：每 token 全量同步 → 每 40ms 一次；1000 delta/s 下从 1000 次降到 25 次。

### 2. stale 与 tick

- 内容 delta 不再 `mark_ao_components_stale`；只有 editor 高度 / dock 行数 / 槽显隐变化才标脏。
- busy tick 16ms → 33ms（30fps）或 50ms（20fps）；spinner 按 c3130 定稿 100ms；内容帧由合批窗口驱动。
- `try_render` 的 16ms cap 保留；AO reproject 快路径恢复。

### 3. streaming tail 缓存

- upper cache 拆成 stable prefix + live tail；tail 单独 generation。
- 每帧只重建 tail；stable prefix 命中缓存。
- `find_stable_markdown_prefix_end`（`paint.rs:207`）用于段落边界；thinking 尾加同款稳定前缀缓存。

### 4. Markdown / Mermaid

- 复用 Markdown 实例（引擎缓存）；流式期 `MermaidFenceMode::Off`，结束切 `Final`。
- 只在稳定段落边界重解析；尾块按纯文本/增量渲染。

### 5. 指纹与 clone

- `entry_fingerprint` 改增量：内容 generation + fold 状态；tool output 用 dirty flag / 长度 + hash 尾，不每帧全量哈希。
- `UiModel` 同步避免全量 clone：`apply_ui_model` 改为借用 / 差量；至少只在结构变化时 clone。
- AO `set_lines` 可选增量 append/replace；本 change 先不做，记录后续。

### 6. 可选引擎增量 API

- 若应用侧仍不足，给 `ApplicationOwnedRuntime` 加 `append_lines` / `replace_tail`；需改 `xylitol-tui` 公共 API，按包 AGENTS「缺能力先改包」。后置。

## 实施任务

1. 合批：`lib.rs` 接受 `MessageUpdate`；取最后快照；40ms 窗口；边界 flush；harness 测试 paint 次数。
2. stale：内容 delta 去掉 `mark_ao_components_stale`；dock 变化才标；测试 AO reproject 恢复。
3. tick：16→33/50ms；spinner 100ms；测试帧率与 CPU。
4. tail cache：stable prefix + tail gen；测试重绘范围。
5. thinking 前缀缓存；Markdown 实例；Mermaid off。
6. fingerprint 增量；UiModel clone 减少。
7. 验证：release lab5 <20%；`lab_ao_stream_delta_perf_report` 帧数/耗时；harness 回归。
8. 可选引擎增量 API（后置）。

## 验证

- release `/tmp/xyl_lab5.py`：1000 delta/s，TUI CPU <20%；tmux server 不升高。
- `lab_ao_stream_delta_perf_report`：paints 约 1/40ms；Σ do_render 降 ≥4×。
- harness：合批不改变最终文本；ToolStart/End/AgentEnd 边界 flush；状态条/spinner 仍按 100ms。
- 真机 tmux：长会话 + 高 token 速率；输入不卡；首字不延迟。
- 回归：`just test-tui`、相关 harness、`just qa`。

## 研究

- tmux CPU 观测与复现装置：[`research/tmux-stream-cpu.md`](./research/tmux-stream-cpu.md)。

## 风险 / 待确认

- 合批可能让 tool args partial 更新延迟 40ms；可接受；边界强制 flush。
- `MessageUpdate` 丢弃中间快照必须保证最后快照是累积的；runtime 已如此（`streaming_message_update` 传 `text_acc` / `thinking_acc` / `tool_calls`）。
- busy tick 降到 33/50ms 可能让 spinner 动画略慢；spinner 100ms，不受影响。
- 全量 clone 的移除可能触及 `Rc<RefCell>` 借用边界；需 harness 护栏。
- 引擎增量 API 后置；若应用侧达不到 <20%，再开。
- release 测量是 macOS debug/release 本机；Windows / 其他终端未验证。
