# Design: c05-rebuild-core

## 整体架构

```
src/
├── agent/
│   ├── loop.rs          # ReAct loop + AgentEvent stream
│   ├── session.rs       # AgentSession (lifecycle, turn mgmt, model/thinking mgmt)
│   ├── model.rs         # XyModel trait + provider registry
│   ├── provider/        # openai.rs, anthropic.rs
│   ├── tools/           # 7 tools + truncation + operations + mutation queue
│   ├── types.rs         # XyContent, XyPart, XyChunk, XyToolSchema
│   ├── traits.rs        # XyModel, XyTool, XyToolCtx
│   └── error.rs         # XyError, XyToolError
├── infra/
│   ├── config/          # YAML loader, three-tier merge
│   ├── hooks/           # Hook dispatcher, script runner
│   └── session/         # SessionManager, SessionEntry, compaction
├── interface/
│   ├── cli/             # clap args
│   └── print.rs         # print mode output
└── lib.rs
```

## 1. Tool System

完全对齐 pi 的 7 个工具。详细设计见之前的 design.md (c05-refactor-tool-system)。

关键决策：
- grep/find 使用外部 `rg`/`fd` 进程 (tokio::process::Command)，非 Rust crate
- TruncationResult 用 `Vec<String>` + byte counting
- FileMutationQueue 用 `tokio::sync::Mutex` per path
- CancellationToken 用 `tokio_util::sync::CancellationToken`

## 2. Session Persistence

对齐 pi 的 JSONL 格式：

```jsonl
{"type":"session","version":3,"id":"...","timestamp":"...","cwd":"...","parentSession":"..."}
{"type":"message","id":"...","parentId":null,"timestamp":"...","message":{...}}
{"type":"compaction","id":"...","parentId":"...","timestamp":"...","summary":"...","firstKeptEntryId":"...","tokensBefore":50000}
{"type":"model_change","id":"...","parentId":"...","timestamp":"...","provider":"openai","modelId":"gpt-4o"}
{"type":"thinking_level_change","id":"...","parentId":"...","timestamp":"...","thinkingLevel":"high"}
```

决策：
- 文件路径: `~/.xylitol/sessions/<id>.jsonl`
- append-only 写入，文件锁 (flock)
- Tree 结构通过 JSONL 中的 parent_session 字段维护
- 不存 `tool_call` 为独立 entry——嵌入在 message 的 tool_calls 数组中

## 3. Agent Session + Thinking Level

对齐 pi 的 AgentSession：

```
AgentSession {
    model_registry: ModelRegistry,
    session_manager: SessionManager,
    tools: ToolRegistry,
    current_model: Option<Arc<dyn XyModel>>,
    thinking_level: ThinkingLevel,  // low | medium | high
    system_prompt: String,
}
```

Thinking Level 映射：
- `low`: 禁用 thinking（或 thinking budget_tokens=0）
- `medium`: 默认 thinking budget
- `high`: 最大 thinking budget
- 切换时 clamp 到 model capabilities

Model 切换：
- `cycle_forward()`: 下一个可用 model
- `cycle_backward()`: 上一个
- `select(model)`: 指定 model
- 切换时 emit `model_select` event, 写入 `model_change` entry 到 session

## 4. Compaction

对齐 pi 的 compaction 流程：
1. 每次 turn 后调用 `should_compact()`, 检查 token 估算值 vs 阈值
2. 若触发：调用 compact() 生成 summary 并替换旧消息
3. 写入 CompactionEntry 到 session JSONL
4. Branch summary 用于 tree navigation 时的上下文桥接

Token 估算策略：
- 简单模式：`len(content) * 0.25` (近似 1 token ≈ 4 chars)
- 精确模式：使用 tokenizer (可选, 默认关闭)

## 5. Config

YAML 格式对齐 pi settings 结构但用 YAML：

```yaml
# ~/.xylitol/config.yaml (global)
default_model: claude-sonnet-4-20250514
compaction_threshold: 0.8
max_iterations: 100

models:
  gpt-4o:
    provider: openai
    model: gpt-4o
    thinking: false
    context_window: 128000
  claude-sonnet:
    provider: anthropic
    model: claude-sonnet-4-20250514
    thinking: true
    context_window: 200000

providers:
  openai:
    base_url: https://api.openai.com/v1
    api_key: $OPENAI_API_KEY
    api: openai-compatible
  anthropic:
    base_url: https://api.anthropic.com
    api_key: $ANTHROPIC_API_KEY
    api: anthropic-messages
```

三层 merge 规则: user > project > global (deep merge, not replace)

## 6. Hook System

升级 hook 点：

| Event | Phase | 用途 |
|-------|-------|------|
| tool_call.{tool} | pre/post | 工具调用拦截 |
| model_query | pre | 模型查询前 |
| **after_provider_request** | post | **修改请求 payload (prefix-caching)** |
| **after_provider_response** | post | **检查响应 headers/status** |
| turn_end | post | 轮次结束通知 |
| message_end | post | 消息完成通知 |
| compaction | post | 压缩完成通知 |
| session_start | post | 会话开始通知 |

### Prefix-Caching Hook 示例 (deepseek v4)

deepseek v4 的 cache_control 需要注入到请求 payload 中：

```json
// hook script 收到 stdin:
{"event": "after_provider_request", "provider": "deepseek", "payload": {...}}
// hook script 修改 payload 并返回:
{"action": "modify", "payload": {...with cache_control added...}}
```

## 7. 移除的模块

- security-policy: 删除 `src/infra/security/` 和 `llmanspec/specs/security-policy/`
- repeat-guard: 删除 `src/agent/repeat.rs` 和 `llmanspec/specs/repeat-guard/`
- planning-orchestrator: 删除 `src/agent/planner.rs` 和 `llmanspec/specs/planning-orchestrator/`
