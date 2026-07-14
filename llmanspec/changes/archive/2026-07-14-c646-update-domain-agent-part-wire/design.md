# Design — c646 AgentPart tagged wire + entry camelCase

## 已锁定决议（用户 1+2）

| ID | 决议 |
|---|---|
| **A1** | Entry 外壳与 header 字段 **全面 camelCase**（`parentId`、`parentSession` 等），对齐 pi |
| **B1** | Thinking 签名键 = `thinkingSignature` |
| **C1** | `toolResult` 关联 id = `toolCallId`（不再 `toolUseId`） |
| **D1** | Image = `{type, data?, url?, mimeType}` |
| **E1** | 非法 content → 该条解释失败/跳过可观测；不整文件炸毁 |
| **F1** | `SESSION_VERSION` → **5**（content+外壳 wire 世代） |
| **G2** | **禁止** `content[]` 内嵌 `type:toolResult`；工具结果只走独立 `role:toolResult` 行 |
| **清库** | 上线前移除 `~/.xylitol/sessions`；无迁移器 |

## 硬约束

1. **JSONL 是 UI 与模型上下文的可恢复真源**。
2. **只认 v5 + 本 design wire**；旧 untagged / snake 外壳 / 裸字符串 **不读、不迁、不猜**。
3. **幂等**：persist → load → parts ≡ 写入；TUI rebuild ≡ 直播 flush 分块。
4. **与 pi 对齐是起点，不是永久绑定**：wire 归 `domain/` 自有类型；日后加 variant / 升 v6 不依赖 pi 包。

## 目标 JSONL 形态（摘录）

```json
{"type":"session","version":5,"id":"…","timestamp":"…","cwd":"…","parentSession":"…"}
{"type":"message","id":"…","parentId":"…","timestamp":"…","message":{
  "role":"assistant",
  "content":[
    {"type":"thinking","thinking":"…","thinkingSignature":"…","redacted":false},
    {"type":"text","text":"…"},
    {"type":"toolCall","id":"…","name":"read","arguments":{}}
  ],
  "api":"…","provider":"…","model":"…","timestamp":0
}}
{"type":"message","id":"…","parentId":"…","timestamp":"…","message":{
  "role":"toolResult",
  "toolCallId":"…",
  "toolName":"read",
  "content":[{"type":"text","text":"…"}],
  "isError":false,
  "timestamp":0
}}
```

非 message 行的 `type` 字符串（`branch_summary` / `bash_execution` 等）**本 change 一并改为 camelCase 判别值以对齐 pi 习惯**（`branchSummary` / `bashExecution` / `modelChange` / `thinkingLevelChange` / `customMessage` / `sessionInfo`），避免外壳半新半旧。字段名一律 camelCase。

## Rust 建模（要调底层 — 推荐 tag union）

**要调。** 不是「多几个 helper」，而是 domain 序列化契约换代。

### 推荐：`#[serde(tag = "type")]` 枚举（tagged union）

```rust
#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum AgentPart {
    Text { text: String },
    Thinking {
        thinking: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        thinking_signature: Option<String>,
        #[serde(default)]
        redacted: bool,
    },
    ToolCall { id: String, name: String, arguments: Value },
    Image {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        data: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        url: Option<String>,
        mime_type: String, // serde → mimeType
    },
    // 无 ToolResult 变体（G2）
}
```

| 方案 | 结论 |
|---|---|
| **Tagged enum（推荐）** | 与 JSON `type` 一一对应；穷尽 match；加新部件 = 新 variant + version 政策 |
| 一堆平行 struct + `enum { Text(T), … }` 手写 | 等价，但重复；无收益 |
| `serde_json::Value` 到底 | 不可取：失去类型闸与幂等测试 |
| 继续 `untagged` | **禁止**（本 change 根因） |

`SessionEntry` **已是** `#[serde(tag = "type")]` — 保留；改 rename 为 camelCase 判别 + `EntryBase`/`SessionHeader` 字段 camelCase（`parent_id`→序列化为 `parentId`）。

`AgentMessage` 已是 `tag = "role"` — 保留；`ToolResultMessage.tool_use_id` 序列化键改为 **`toolCallId`**（`#[serde(rename = "toolCallId")]`）。

### 日后可扩展性（脱离 pi 生态之后）

**可以，且这是正确底座：**

1. **判别式 wire**（`type` / `role`）是跨语言、可版本化的开放扩展点；不绑定 pi npm 包。
2. **新能力** = 新 `AgentPart` / `SessionEntry` variant + bump `SESSION_VERSION`（或同版本 + 「未知 type 跳过」政策，另 change 定）。
3. **自有方向**：domain 类型是 SSOT；pi 只是 v5 的对齐参考。v6 起可加 xylitol 专有 part（如 `diffHunk`、`skillInvocation`），不必等 pi。
4. **未知部件政策（建议写进 future，本 change 不实现）**：`#[serde(other)]` 或 `Unknown { type, rest }` 用于前向兼容读；**写出仍只写已知**。本 change 清库 + 严格已知集合即可。

**风险可控点**：枚举闭集会在加 variant 时强迫改 match — 这是优点（编译期发现漏投影到 UI/provider）。

## 旧格式

不读。上线前删 `~/.xylitol/sessions`。E1：单条解释失败不影响整文件 JSONL 行扫描。

## 实现顺序

1. `SESSION_VERSION = 5`；`EntryBase` / `SessionHeader` / `SessionEntry` 判别 camelCase；字段 `parentId` / `parentSession`
2. `AgentPart` tagged；去掉嵌套 `ToolResult`；`Image`/`Thinking` 字段对齐
3. `AgentMessage::ToolResultMessage` → `toolCallId`
4. `message_text` 只聚合 text；converter / fixture / provider 手写 JSON
5. TUI `session_entry_to_ui_entries`；travel/fork 共用
6. 测试闸全绿

## 验证矩阵

| 闸 | 断言 |
|---|---|
| domain | Thinking/Text/ToolCall round-trip；旧 untagged 失败；JSON 含 `parentId`/`thinkingSignature`/`toolCallId` |
| agent | persist→load 部件分离 |
| TUI harness | rebuild 含 `UiEntry::Thinking` |
| version | 新文件 header `version: 5` |

## Out of scope

- 未知 `type` 前向兼容读取（future）
- 树 hint / qwen E2E
- 迁移工具
