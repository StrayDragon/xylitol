# c20-add-tools — Design

## Context

- PRD: §0.3（adk-tool FunctionTool 对标）、§7.5（AI Patch Apply 策略）、§0.6（rtk 集成方式）
- 依赖关系见 proposal.md frontmatter（depends_on / blocks 为 SSOT）

## Goals / Non-Goals

### Goals

- 定义 Tool trait，兼容 adk-core 的 FunctionTool 接口
- 实现 7 个内置工具（read, bash, edit, write, grep, find, ls）
- 实现 AI Patch Apply 三级策略（fudiff → patch → 错误回退）
- ToolRegistry 工具注册与查找
- bash 工具集成 rtk 管道压缩（feature-gated）

### Non-Goals

- 不实现 MCP 远程工具（c65 负责）
- 不实现 diff 审查 UI（c75 负责）
- 不实现 hooks 拦截逻辑（c40 负责），仅在关键点预留事件触发位置
- 不实现 sandbox 隔离执行（c50 负责）

## Decisions

### Decision 1: Tool trait 接口设计

```mermaid
classDiagram
    class Tool {
        <<trait>>
        +name() &str
        +description() &str
        +parameters() JsonSchema
        +execute(ctx, args) Future~Result~ToolOutput~~
    }

    class ToolOutput {
        +String output
        +bool success
        +Option~String~ error
    }

    class ToolContext {
        +AppConfig config
        +PathBuf project_root
        +Sender~ToolEvent~ event_tx
    }

    class ToolRegistry {
        -Map~String, Box~dyn Tool~~ tools
        +register(tool) void
        +get(name) Option~&dyn Tool~
        +list() Vec~&str~
    }

    class ReadTool
    class BashTool
    class EditTool
    class WriteTool
    class GrepTool
    class FindTool
    class ListTool

    Tool <|.. ReadTool
    Tool <|.. BashTool
    Tool <|.. EditTool
    Tool <|.. WriteTool
    Tool <|.. GrepTool
    Tool <|.. FindTool
    Tool <|.. ListTool
    ToolRegistry o-- Tool
```

**选择**: 自定义 `Tool` trait 而非直接使用 adk-core `FunctionTool`，但保持接口等价以便后续通过适配器桥接。

**适配器策略**: 后续集成 adk-rust 时，`Tool` trait 可通过 wrapper 实现 `FunctionTool`：

```
XylitolTool(impl Tool) → impl FunctionTool  // 适配器
FunctionTool(外部) → impl Tool              // 反向适配（MCP 工具等）
```

**权衡**: 自定义 trait 避免在工具层直接依赖 adk-core 类型（降低耦合），但需要一个薄适配层。考虑到 agent loop 层才会用到 adk-core，这个分层是合理的。

### Decision 2: 7 个内置工具的参数与行为

```mermaid
graph TD
    subgraph "文件操作工具"
        READ["read<br/>file: PathBuf<br/>offset?: u32<br/>limit?: u32"]
        WRITE["write<br/>file: PathBuf<br/>content: String<br/>create_dirs?: bool"]
        EDIT["edit<br/>file: PathBuf<br/>old: String<br/>new: String<br/>replace_all?: bool"]
    end

    subgraph "搜索工具"
        GREP["grep<br/>pattern: String<br/>path?: PathBuf<br/>file_type?: String<br/>context?: u32"]
        FIND["find<br/>pattern: String<br/>path?: PathBuf<br/>type?: file|dir"]
        LIST["ls<br/>path?: PathBuf<br/>all?: bool<br/>long?: bool"]
    end

    subgraph "执行工具"
        BASH["bash<br/>command: String<br/>timeout?: u32<br/>cwd?: PathBuf"]
    end
```

**edit 工具选择 old/new 搜索替换模式**（与 codex 一致），而非行号模式。理由：
- AI 模型生成行号经常偏移，old/new 文本匹配更可靠
- old/new 可直接映射为 unified diff 的 hunk（用于 patch apply）
- `replace_all` 支持批量替换

**bash 工具关键行为**:
- 超时控制（默认 120s，可通过参数覆盖）
- 输出截断（超过限制时截断并标注 `[truncated]`）
- rtk 管道压缩（feature flag `rtk` 门控，通过管道 `command | rtk` 实现）
- 工作目录默认为 `project_root`

### Decision 3: Patch Apply 三级策略

```mermaid
flowchart TD
    INPUT["代理生成 unified diff"] --> PARSE["解析 diff hunk"]

    PARSE --> STRATEGY{"配置策略?"}

    STRATEGY -->|fudiff| FUZZY["fudiff 模糊匹配<br/>容忍 max_line_offset"]
    STRATEGY -->|patch| EXACT["patch crate 精确匹配"]
    STRATEGY -->|hybrid| HYBRID["先 fudiff → 失败回 patch"]

    FUZZY --> RESULT1{"应用结果?"}
    RESULT1 -->|成功| WRITE["写入文件系统"]
    RESULT1 -->|失败| FALLBACK1{"retry_on_failure?"}
    FALLBACK1 -->|yes & hybrid| EXACT
    FALLBACK1 -->|yes & !hybrid| RETRY["返回错误 → 代理重新生成"]
    FALLBACK1 -->|no| RETRY

    EXACT --> RESULT2{"应用结果?"}
    RESULT2 -->|成功| WRITE
    RESULT2 -->|失败| RETRY2{"重试次数 < max_retries?"}
    RETRY2 -->|yes| RETRY
    RETRY2 -->|no| ERROR["返回 PatchApplyError<br/>含原始 diff + 失败原因"]

    WRITE --> HOOK{"pre.tool_call.file_write<br/>hook 存在?"}
    HOOK -->|无 hook| DONE["变更生效"]
    HOOK -->|有 hook| APPROVAL{"hook 返回<br/>requires_approval?"}
    APPROVAL -->|yes| REVIEW["进入轻量确认<br/>（c75 完整审查）"]
    APPROVAL -->|no| DONE

    style HYBRID fill:#e8f5e9
    style ERROR fill:#ffebee
```

**选择**: `hybrid` 作为推荐策略（fudiff 优先 + patch 兜底），因为 AI 生成的 diff 最常见的问题是行号偏移，fudiff 专为此场景设计。

**PatchApplyConfig 映射**（来自 c10）:
```yaml
patch_apply:
  strategy: "hybrid"          # fudiff | patch | hybrid
  max_line_offset: 50          # fudiff 最大容忍偏移
  retry_on_failure: true       # 失败是否触发代理重生成
  max_retries: 2               # 最大重试
```

**权衡**: hybrid 比 pure fudiff 多一次 fallback 开销，但显著降低应用失败率。fudiff 极早期（v0.0.x），可能需要调整容忍度参数。

### Decision 4: rtk 管道集成方式

```mermaid
flowchart LR
    BASH_CMD["bash tool<br/>接收 command"] --> CHECK{"feature rtk<br/>启用?"}
    CHECK -->|no| DIRECT["直接执行<br/>command"]
    CHECK -->|yes| RTK_CHECK{"rtk 可用?"}
    RTK_CHECK -->|no| DIRECT
    RTK_CHECK -->|yes| PIPE["command 2>&1 | rtk<br/>管道压缩"]
    DIRECT --> CAPTURE["捕获 stdout/stderr"]
    PIPE --> CAPTURE
    CAPTURE --> TRUNCATE["输出截断<br/>超限时标注"]
    TRUNCATE --> RETURN["返回 ToolOutput"]
```

**选择**: 管道模式（`command | rtk`）而非库模式。rtk 当前是外部二进制，库 feature flag 尚未贡献。

**权衡**: 管道模式需要 rtk 在 PATH 中可用，但不需修改 rtk 代码。未来 rtk 提供 library feature 后可切换为进程内压缩（消除子进程开销）。

## Risks / Trade-offs

| 风险 | 等级 | 缓解 |
|------|------|------|
| fudiff 极早期（v0.0.x），API 可能不稳定 | 高 | 封装 PatchApplier trait，fudiff 仅作为实现细节；hybrid 策略下 patch 兜底保底 |
| Tool trait 与 adk-core FunctionTool 签名未来不兼容 | 中 | 保持接口等价设计，适配器层隔离变更 |
| bash 工具的安全边界（命令注入风险） | 高 | 安全策略由 c50 统一管控（forbidden_patterns、sandbox），本 change 仅执行 |
| edit 工具 old/new 匹配在多行重复时歧义 | 低 | `replace_all=false` 时仅替换首个匹配；匹配失败时返回错误让代理重试 |

### 待确认问题

- rtk 库模式时间线——如果 rtk 短期内不提供 library feature，是否仍保持管道模式？
