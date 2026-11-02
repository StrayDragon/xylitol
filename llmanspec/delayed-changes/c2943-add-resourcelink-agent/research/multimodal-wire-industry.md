# 调研：主流 agent 在 JSON-RPC / 类 JSON-RPC 下如何处理大图、视频与大载荷

> 背景：xylitol 线协议 `MAX_BODY_BYTES = 4 MiB`、`MAX_BATCH_ITEMS = 100`；`infra-image` 多模态 base64 上限约 **4.5MB**；远程 `prompt` 目前仅文本。本稿供未来「读图 / 读视频 / 远程 attach 多模态」增强时对照，**非现行产品承诺**。

## 1. 「JSON-RPC」往往只占一层

| 产品 / 协议 | 传输形态 | JSON-RPC 用在哪 | 图 / 音 / 大对象通常走哪 |
|-------------|----------|-------------------|---------------------------|
| **OpenAI Codex App Server** | stdio / WS **NDJSON**（一行一帧） | 整线 JSON-RPC 2.0 | 单条 JSONL 可含大 tool 输出 / 生成图；**服务端 64 MiB/消息**，客户端默认 **32 MiB/行**（可调 `AppServerLimits::max_line_bytes`） |
| **Gemini CLI（ACP）** | `--acp`：NDJSON + JSON-RPC 2.0 | IDE ↔ Agent 控制 | `session/prompt` 的 **ContentBlock**（`image`/`audio` = base64 + mimeType）；读盘走 **客户端 FS 代理**（`fs/read_text_file`，文本） |
| **Claude Code** | **stream-json**（NDJSON），主通道非 JSON-RPC | MCP 嵌在 `sdk_control_request` 里的 `mcp_message` | 用户图在 stdin **user message** 的 `image` + `base64`；SDK 校验约 **15MB base64** 量级 |
| **ACP（Zed / 通用）** | JSON-RPC | `session/prompt` / `session/update` / tool content | 基线 **text + resource_link**；**image/audio 需 capability 显式开启**；与 MCP ContentBlock 对齐 |
| **MCP** | JSON-RPC | tools / resources | `resources/read` 返回 **blob (base64)** 或 **ResourceLink**（只传 URI） |
| **Continue** | 自研 `messageType` + `data`（**非 JSON-RPC**） | IDE ↔ Core | 图在 **Provider HTTP**（Anthropic `image` block）；Agent 工具多经 **MCP** |
| **LSP** | JSON-RPC + **Content-Length** 分帧 | 语言服务 | **不支持 batch**；大 payload 靠按字节读满一帧，不把二进制当常规载荷 |

参考：

- [Codex App Server Overview](https://www.mintlify.com/openai/codex/api/overview)
- [Codex exec-server 64 MiB 消息上限](https://github.com/openai/codex/blob/main/codex-rs/exec-server/src/connection.rs)（`MAX_STDIO_JSONRPC_MESSAGE_LEN`）
- [Codex thread/resume 单条 23MB+ JSONL 与 `excludeTurns`](https://github.com/openai/codex/issues/40362)
- [Codex 客户端默认 32 MiB/行说明（D0144）](https://docs.rs/openai-rs-codex/latest/src/openai_rs_codex/app_server/client.rs.html)（`DEFAULT_LINE_LIMIT`）
- [ACP Content（image/audio base64）](https://agentclientprotocol.com/protocol/v2/content)
- [Gemini CLI ACP mode](https://geminicli.com/docs/cli/acp-mode/)
- [Claude Code stream-json / MCP 嵌套 JSON-RPC](https://github.com/Roasbeef/claude-agent-sdk-go/blob/main/docs/cli-protocol.md)
- [MCP resources/read blob](https://modelcontextprotocol.io/specification/2025-11-25/server/resources)
- [LSP Content-Length](https://microsoft.github.io/language-server-protocol/specifications/specification-3-15/)

## 2. 五种常见策略（可组合）

### A. 控制面 / 数据面分离

- Client ↔ Host：小 JSON-RPC（prompt、subscribe、审批）。
- Host ↔ Model：HTTP(S) 内联 base64 image blocks。
- 工具读图：Host 进程内 `read` → resize → `AgentPart::Image`，**不**要求用户经 POST 上传原图。

xylitol 现状与此一致（远程 `prompt` 无图；Read 在 Host + `infra-image`）。

### B. 路径 / 链接，不传字节（ACP + MCP 推荐）

- `ContentBlock::ResourceLink`（URI + mime + size）。
- MCP `file://` 资源；工具可返回 link 而非 blob。

适合 **4 MiB** 线协议与远程 attach。

### C. 仍 inline base64，但抬高「单消息 / 单行」上限

- Codex：**64 MiB（服务） / 32 MiB（客户端默认）** 每 JSONL 记录。
- 动机：单 turn replay、生成图、大 tool 输出会到 **数 MiB～数十 MiB**；过小会误杀合法会话。

若 xylitol 要在 **journal/resync 或单帧 history** 里内联媒体，4 MiB 会偏紧；行业对策是 **提限 + 脱敏**，而非仅调 batch。

### D. 同步 / resume 不背媒体仓库

- Codex 默认 `thread/resume` 内联生成图 → 有界客户端无法恢复；缓解 **`excludeTurns: true`**（元数据 resume）。

xylitol 类似：`session/event` journal + tool 预览 **`[image]`**，避免 WS 拖全量 base64。

### E. 工具层 / UI 截断（与 RPC 限额正交）

- Zed：大文件 outline + 按行读；图在 read_file 工具内处理。
- ACP terminal output：超字节上限从开头截断（合法 UTF-8）。
- **视频**：公开协议未见整段 mp4 base64 进 JSON-RPC；预期 **抽帧 → image**、**path 引用**、或 **Provider 原生 video API（另一条 HTTP）**。

## 3. 与 xylitol 现行数字的对照

| 项 | xylitol | 行业常见 |
|----|---------|----------|
| POST/WS 单帧 | **4 MiB** | Codex 类 **32–64 MiB/行**；LSP 按 Content-Length 无固定 4M |
| 多模态准备 | **~4.5MB base64**（`ImageResizeOptions`） | Claude SDK ~15MB base64；与线协议宜 **留 JSON 开销** |
| Batch | **100** | LSP 禁 batch；MCP/Codex 另有背压 |
| 远程 prompt 贴图 | **未做** | ACP `session/prompt` + capability |
| 读视频 | **未做** | 不应走同一 JSON-RPC 整文件通道 |

已知张力：**4.5MB（r1443）> 4 MiB（`MAX_BODY_BYTES`）**——若未来 RPC 内联整图，须对齐常量或改用 ResourceLink。

## 4. 对 xylitol 增强面的方向性结论（供 propose 时选用）

1. **保持 4 MiB 作控制面** + 多模态默认 **Host 工具 + Provider HTTP**（延续现状）。
2. **远程多模态**：优先 **ResourceLink / workspace path / upload id**，而非 giant `prompt` params。
3. **若要对齐 ACP client**：评估单帧 **16–32 MiB**（与 Codex 客户端同量级），且 **journal/resync 禁止内联生成媒体字节**。
4. **读视频**：单独能力——Host 抽帧/转码 + path/元数据；**不**扩大 JSON-RPC 到 GB 级。
5. **实现债（与 TUI 相关，见 proposal 附录）**：`tool_exec` 对 event/history 使用 `parts_preview_text`（`[image]`），可能未把 `AgentPart::Image` 完整送入 `ToolResultMessage.content`；增强时应与 `agent-tools` r1145 一并核对。

## 5. xylitol 线协议限额代码索引（现行）

| 限制 | 常量 / 入口 |
|------|-------------|
| 4 MiB body/WS | `crates/xylitol-serve/src/wire.rs` `MAX_BODY_BYTES`；`listen.rs` |
| batch 100 | `crates/xylitol-protocol/src/codec.rs` `MAX_BATCH_ITEMS` |
| 深度 2048 | `codec.rs` `JSON_DEPTH_LIMIT` |
| 图 resize | `plugins/.../image/resize.rs` 默认 2000×2000、4_500_000 bytes base64 |
| Journal | `occupancy/ws.rs` `DEFAULT_JOURNAL_CAPACITY = 10_000` |

Spec：`protocol-app` r1927/r2917/r2918；`server-core` r1804；`infra-image` r1443。
