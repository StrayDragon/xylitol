---
depends_on: []
title: 远程多模态与线协议大载荷（ResourceLink / 帧上限）增强面
needs_specs_change: true
---

# 远程多模态与线协议大载荷（ResourceLink / 帧上限）增强面

## Why

- 现行线协议 **4 MiB / batch 100** 适合本机 Host 控制面；`infra-image` 多模态上限 **~4.5MB** 与线协议存在数字张力。
- 产品方向若包含 **远程 attach 贴图、读视频、历史 replay 带媒体**，需要对齐主流 agent（ACP / Codex / MCP）的 **ResourceLink、单帧 MiB 级上限、resume 脱敏** 等做法，而非仅放大 `MAX_BODY_BYTES`。
- 本 change 为 **purpose-draft**：记录调研与候选项，落实时走 `llman-sdd-propose`。

## What Changes（候选项，promote 时逐条重估）

- **协议 / spec**：`protocol-app` / `server-core` 补充多模态载荷策略（path 引用 vs inline base64、单帧上限与 `infra-image` 关系、视频禁止整文件进 JSON-RPC）。
- **线协议**：可选抬高 `MAX_BODY_BYTES` 或引入 **ResourceLink** 式 `prompt` 参数；WS/journal **不**内联全量 base64（延续或强化 `[image]` 预览语义）。
- **TUI / runtime**：读图后 UI 与 history 是否保留 `AgentPart::Image`（见下文「当前 TUI 读图逻辑」与 research §4）。
- **读视频**：Host 侧抽帧或元数据能力（独立 scope，不与本 change 默认同批交付）。

## 调研落点

- `llmanspec/changes/c2943-add-resourcelink-agent/research/multimodal-wire-industry.md`（主流 agent 对照、策略归纳、代码索引）。

## 非目标（草案阶段）

- 不修改现行 4 MiB / batch 100 行为。
- 不实现读视频、不开 PR。

## 附录：当前 TUI「读图」之后逻辑（as-is，2026-10-22）

以下描述 **Agent 调用 Read 工具读图片文件** 后，产品 TUI 侧可见行为（含 in-process 与 remote attach，事件形状相同）。

### 1. 工具执行（Host / runtime，TUI 不参与读盘）

1. `Read` 识别图片路径 → `image_content_from_path`（resize，默认 max 2000×2000、base64 ≤ 4_500_000）→ `AgentPart::text(note)` + `AgentPart::Image`（`plugins/xylitol-official-plugin-tools/src/tools/read.rs`）。
2. `tool_exec::run_one` 走 `execute_as_parts`，得到 parts 后生成 **预览字符串** `parts_preview_text`：文本 note + 行 `[image]`（`crates/xylitol-runtime/src/agent/runtime/tool_exec.rs`）。
3. 发出 `ToolExecutionStart` / `ToolExecutionUpdate`（若未流式则 Update 一次）/ `ToolExecutionEnd`，**`result` 字段均为该预览字符串**（非 JSON、不含 base64）。
4. 写入会话 history 时，`quiet_write_edit_for_history` 对 `read` **未**特殊处理：history content 为 **纯文本 preview**（`"Read image file [size]\n[image]"`），**不**保留 `AgentPart::Image` parts（与 `agent-tools` r1145 在工具端口层的 parts 语义存在张力，增强时应核对）。

### 2. TUI bridge（滚动区 / 工具块）

1. `apply_xy_event` → `ToolExecutionEnd` → `apply_tool_result_to_entries`（`crates/xylitol-app-tui/src/bridge/mod.rs`）。
2. `humanize_tool_result_for_tui("read", …)` 依赖 **JSON** `{"content":…}` 形状（`preview.rs::humanize_read_tool_output`）；读图结果 **不是 JSON**，人性化失败。
3. 工具块 `output` 最终为 **原始 preview 字符串**（通常 `"Read image file [2.0KB]\n[image]"`），**不渲染位图、不显示 base64**。
4. 线协议 remote 路径：`session/event` 携带的 tool result 同样是上述字符串（wire 不扩图字节）。

### 3. 与用户贴图（clipboard）区分

- **粘贴图片**（非 Read 工具）：`effects/pending_ui/clipboard.rs` → `XyDriver::stage_clipboard_image` → 临时文件路径 **插入编辑器**（`driver/clipboard.rs` + `clipboard/image.rs`），用户提交 prompt 时仍是 **文本路径字符串**，**不经 RPC 内联图片**（remote `prompt` body 仅 `message` 等字段）。

### 4. 小结

| 环节 | 读图后 TUI 用户看到什么 | 模型是否收到真图 |
|------|-------------------------|------------------|
| Read 工具成功 | 工具行文案 + `[image]` 占位 | **待核实**：history 当前为 preview 文本，provider 路径需对照 `tool_exec` + `anthropic_messages` 是否另有恢复 |
| 粘贴图 | 编辑器里的绝对路径 | 取决于 prompt 文本是否被模型当路径处理（无专用 multimodal prompt RPC） |

增强面应明确：TUI **展示**（缩略图 / 路径）、**线协议**（ResourceLink）、**history parts**（r1145）三条线是否一并改。
