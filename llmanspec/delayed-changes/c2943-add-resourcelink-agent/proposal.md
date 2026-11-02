---
depends_on: []
title: ResourceLink 远程媒体引用（线协议大载荷）
needs_specs_change: true
---

# ResourceLink 远程媒体引用（线协议大载荷）

## Why

- 远程 attach 贴图与历史 replay 带媒体若继续内联 base64，会把 **4 MiB / batch 100** 的线协议信封撑爆；主流 agent（ACP / Codex / MCP）用 **ResourceLink**（路径 / URI 引用）而不是放大单帧。
- provider 侧媒体投影已完善：OpenAI 族图片投影 c2993、三族协议保真 c3012–c3019 已落地，剩余缺口只在「远程媒体如何引用」这一条线。
- 本 change 为 **purpose-draft**：只保留 ResourceLink 候选，落实时走 `llman-sdd-propose`。

## What Changes（候选项，promote 时逐条重估）

- **线协议**：`prompt` 参数引入 **ResourceLink** 形态的媒体引用（path / URI + 媒体类型），WS/journal **不**内联全量 base64，延续 `[image]` 预览语义。
- **协议 / spec**：`protocol-app` / `server-core` 补充 ResourceLink 载荷策略：引用解析归属 Host、客户端不得直接读远端文件、缺失引用 须可观测失败。
- **TUI / runtime**：remote attach 贴图走 ResourceLink 时，端侧只传引用，Host 侧解析为 `AgentPart::Image` 后再投影给模型（history parts 形状已由 c2945 / r1145 落地）。

## 已拆出

- 2026-10-29：切片 A（OpenAI 族图片投影）由 `c2993-add-openai-image-projection` 落地。
- 2026-10-30：provider 协议保真（thinking 签名 / 终止语义 / prompt caching / 缓存亲和 / 输出预算 / 错误分类 / 结构化输出）由 c3012–c3019 落地；本草案不再承担 provider 侧工作。
- 本草案只保留 ResourceLink；单帧上限放大与读视频不再随本 change 交付。

## 调研落点

- `llmanspec/changes/c2943-add-resourcelink-agent/research/multimodal-wire-industry.md`（主流 agent 对照、策略归纳、代码索引）。

## 非目标（草案阶段）

- 不修改现行 4 MiB / batch 100 行为；不抬高 `MAX_BODY_BYTES`。
- 不实现读视频（独立 feature request）。
- 不改 provider 侧媒体投影（已由 c2993 / c3012–c3019 交付）。
- 不开 PR。

## 附录：当前 TUI「读图」之后逻辑（as-is，2026-10-27 复核）

以下描述 **Agent 调用 Read 工具读图片文件** 后，产品 TUI 侧可见行为（含 in-process 与 remote attach，事件形状相同）。

### 1. 工具执行（Host / runtime，TUI 不参与读盘）

1. `Read` 识别图片路径 → `image_content_from_path`（resize，默认 max 2000×2000、base64 ≤ 4_500_000）→ `AgentPart::text(note)` + `AgentPart::Image`（`plugins/xylitol-official-plugin-tools/src/tools/read.rs`）。
2. `tool_exec::run_one` 走 `execute_as_parts`，得到 parts 后生成 **预览字符串** `parts_preview_text`：文本 note + 行 `[image]`（`crates/xylitol-runtime/src/agent/runtime/tool_exec.rs`）。
3. 发出 `ToolExecutionStart` / `ToolExecutionUpdate`（若未流式则 Update 一次）/ `ToolExecutionEnd`，**`result` 字段均为该预览字符串**（非 JSON、不含 base64）。
4. 写入会话 history 时 **保留全部 parts 含 `AgentPart::Image`**：c2945 起 quiet 摘要只在结果仍是单段文本且工具为 `write` / `edit` 时生效，`read` 走 `_ => (result.0.clone(), None)` 分支，图片分片不被拍平（`agent-tools` r1145 由此落地；早期「history 只剩 preview 文本」的描述已失效）。

### 2. TUI bridge（滚动区 / 工具块）

1. `apply_xy_event` → `ToolExecutionEnd` → `apply_tool_result_to_entries`（`crates/xylitol-app-tui/src/bridge/mod.rs`）。
2. `humanize_tool_result_for_tui("read", …)` 依赖 **JSON** `{"content":…}` 形状（`preview.rs::humanize_read_tool_output`）；读图结果 **不是 JSON**，人性化失败。
3. 工具块 `output` 最终为 **原始 preview 字符串**（通常 `"Read image file [2.0KB]\n[image]"`），**不渲染位图、不显示 base64**。
4. 线协议 remote 路径：`session/event` 携带的 tool result 同样是上述字符串（wire 不扩图字节）。

### 3. 与用户贴图（clipboard）区分

- **粘贴图片**（非 Read 工具）：`effects/pending_ui/clipboard.rs` → `XyDriver::stage_clipboard_image` → 临时文件路径 **插入编辑器**（`driver/clipboard.rs` + `clipboard/image.rs`），用户提交 prompt 时仍是 **文本路径字符串**，**不经 RPC 内联图片**（remote `prompt` body 仅 `message` 等字段）。

### 4. 小结（已核实）

| 环节 | 读图后 TUI 用户看到什么 | 模型是否收到真图 |
|------|-------------------------|------------------|
| Read 工具成功（**Anthropic 族**） | 工具行文案 + `[image]` 占位 | **收到**：`anthropic_messages.rs` 两处把 `AgentPart::Image` 转为 `{"type":"image","source":{"type":"base64",…}}` |
| Read 工具成功（**OpenAI 族**） | 工具行文案 + `[image]` 占位 | **已收到**：c2993 起 Responses 投影 `input_image`，Completions 用 `image_url` 并在 tool 图片时补发紧邻 user 消息 |
| 粘贴图 | 编辑器里的绝对路径 | 取决于 prompt 文本是否被模型当路径处理（无专用 multimodal prompt RPC） |

增强面应明确：TUI **展示**（缩略图 / 路径）、**线协议**（ResourceLink）、**history parts**（r1145，已落地）三条线是否一并改；**OpenAI 族图片投影** 已由 c2993 落地，不再作为本草案前置缺口。
