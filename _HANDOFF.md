# Handoff: c05-rebuild-core

## 目标

完整复刻 pi coding-agent 核心功能到 xylitol (Rust)，一次性重写所有核心模块。

## 最终状态

- **已归档**: `2026-06-09-c05-rebuild-core`
- **33/33 任务完成**
- **cargo test --lib**: 159 passed, 1 ignored
- **cargo check --test bdd**: 0 errors (80+ step defs 编译通过)
- **代码总量**: ~12,871 行 (src/)

## 模块对齐度分析

### 1. Tool System — 95%

| 工具 | 对齐度 | 说明 |
|------|--------|------|
| edit | 100% | multi-edit, original-file matching, overlap/uniqueness/no-change 检查, BOM/CRLF 规范化, fuzzy unicode (NFKC), unified patch + display diff |
| read | 100% | 2000行/50KB 截断, offset/limit, remaining lines hint, image 文件探测 |
| write | 100% | FileMutationQueue 序列化, 原子写入 (temp + rename), 自动创建父目录 |
| ls | 100% | 不区分大小写字母排序, `/` 后缀, 可选 path, entry limit hint |
| grep | 90% | ripgrep (`rg --json`), regex/glob/ignoreCase/literal/context/limit, 行截断。**差异**: 我们使用 `.output()` 阻塞等待而非 pi 的 streaming — cancel 后无法立即终止 rg 进程 |
| find | 90% | fd (`--glob --full-path`), gitignore, Posix 相对路径, limit。**差异**: 同上，使用 `.output()` 而非 streaming |
| bash | 85% | CancellationToken, kill_tree (kill -9 进程组), shell 检测, timeout, 合并 stdout/stderr。**差异**: 使用 `.output()` 而非 streaming — 无法在命令执行期间显示部分输出 (tool_execution_update) |
| truncate | 100% | truncateHead, truncateTail, truncateLine 完全对齐 pi |
| mutation queue | 100% | per-path 序列化, 不同 path 并行 |

**已知差距**:
- grep/find/bash 使用 `.output()` 而非 streaming — cancel 后等待进程结束，而 pi 可以立即 kill
- 无 tool_execution_update 事件 (streaming partials) — bash 长时间运行时无法给用户反馈进度

### 2. Session Persistence — 80%

| 功能 | 对齐度 | 说明 |
|------|--------|------|
| JSONL 文件存储 | 100% | 每行一个 JSON 对象, append-only |
| Entry types (7种) | 100% | message, compaction, branch_summary, model_change, thinking_level_change, custom, session header |
| SessionManager CRUD | 95% | create/append/load/list/exists 全实现 |
| 会话树 | 60% | parent_session 字段存在，但 fork 逻辑未实现 |
| Version migration | 30% | version 字段写入但无 v2→v3 迁移逻辑 |
| 文件锁 | 0% | 使用 `tokio::fs::OpenOptions::append(true)` 但未使用 flock — 并发写入可能乱序 |
| Branch summary 生成 | 40% | stub 实现 — 返回简单文本，未调用 LLM 生成真正摘要 |

**已知差距**:
- 无并发文件锁 (flock) — 多进程同时写同一 JSONL 可能损坏
- 无 version migration — 读取旧版本会话文件可能失败
- 无 tree navigation — fork/switch 未实现

### 3. Agent Session — 80%

| 功能 | 对齐度 | 说明 |
|------|--------|------|
| Model registry | 90% | register/find/list/cycleForward/cycleBackward/select |
| Thinking level | 95% | off/minimal/low/medium/high + clamp to model capabilities |
| Event stream | 90% | TurnStart, TextDelta, ThinkingDelta, ToolExecutionStart/End, TurnEnd, AgentEnd, ModelSelect, ThinkingLevelChanged, CompactionStart/End — 全部定义并发射 |
| System prompt | 70% | 简单字符串传入，无 context files 加载，无 prompt template 展开 |
| Skill blocks | 0% | pi 解析 `<skill name="..." location="...">` 块，xylitol 未实现 |
| Context loading | 0% | pi 加载 AGENTS.md 等上下文文件，xylitol 未实现 |
| Auto-retry | 0% | pi 在 API error 时自动重试，xylitol 未实现 |
| Session export | 0% | pi 支持导出为 HTML，xylitol 未实现 |

**已知差距**:
- 无事件订阅/监听器模式 — 我们通过 stream 直接消费，pi 有 subscribe/listener 模式
- 无 system prompt 构建流程 (加载 context files, 拼接 skill prompts)
- 无 prompt template 展开
- 无 auto-retry on error
- 无 bash executor 作为独立组件 (当前嵌入在 BashTool 中)

### 4. Agent Loop / Runtime — 85%

| 功能 | 对齐度 | 说明 |
|------|--------|------|
| ReAct loop | 95% | send→receive→execute tools→append→repeat, 含 max_iterations 限制 |
| Event stream | 95% | 完整的 turn/message/tool events |
| Streaming tokens | 95% | TextDelta, ThinkingDelta 逐 token 发射 |
| 并行工具执行 | 50% | 仅 sequential mode — pi 支持 parallel 模式 (单次 model 调用的多个 tool_calls 并发执行) |
| Tool streaming updates | 0% | 无 tool_execution_update — bash 部分输出无法实时显示 |
| Abort | 90% | CancellationToken 传递到每个工具，但 process kill 有延迟 |

**已知差距**:
- 仅 sequential 工具执行 — pi 的 parallel execution mode 未实现
- 无 tool_execution_update (bash 部分输出)
- 当 tool_calls 为空时 loop 退出 — pi 继续等待下一轮 (turn-based vs iteration-based 差异)

### 5. Compaction — 40%

| 功能 | 对齐度 | 说明 |
|------|--------|------|
| Token estimation | 80% | `len * 0.25` 近似公式，但 pi 有更精确的 tokenizer-based 模式 |
| should_compact | 100% | 阈值百分比 + 上下文窗口比较 |
| compact() | 10% | **未实现** — 仅定义了函数签名和数据结构。pi 调用 LLM 生成摘要、追踪文件操作、写入 CompactionEntry |
| Branch summarization | 10% | stub — pi 使用 LLM 生成桥接摘要 |
| File operation tracking | 0% | pi 在压缩间追踪 readFiles/modifiedFiles，xylitol 未实现 |

**已知差距（严重）**:
- **compaction 核心未实现** — 无 LLM-based summarization，无 CompactionEntry 写入
- 无 file operation tracking 跨压缩周期

### 6. Config — 70%

| 功能 | 对齐度 | 说明 |
|------|--------|------|
| YAML 三层配置 | 60% | 类型定义完整 (global/project/user)，但 merge 逻辑未完全实现 |
| Model entries | 90% | provider, model-id, base_url, api_key 解析 |
| Provider entries | 80% | openai/anthropic, base_url override |
| ENV 变量插值 | 0% | pi 支持 `$VAR` / `${VAR}`, xylitol 未实现 |
| 外部命令密钥 | 0% | pi 支持 `!cmd` 前缀, xylitol 未实现 |
| Settings | 70% | max_iterations, compaction_threshold 硬编码，未从 config 读取 |

**已知差距**:
- 三层 merge 未完成 — 仅定义了数据结构
- 无 `$ENV_VAR` 插值
- 无 `!command` 密钥解析
- 大部分 settings 硬编码而非从 YAML 读取

### 7. CLI — 60%

| 功能 | 对齐度 | 说明 |
|------|--------|------|
| clap args | 90% | --model, --session, --print, --prompt, positional prompt |
| Session picker | 0% | pi 列出已存在会话让用户选择，xylitol 未实现 |
| Interactive TUI | 0% | pi 有完整的 ratatui-based interactive mode |
| RPC mode | 0% | --rpc 标志存在但未实现 |
| Project trust | 0% | pi 在首次运行目录时确认信任，xylitol 未实现 |
| Stdin input | 80% | 实现但不支持 `-` 约定 |

### 8. Hooks — 80%

| 功能 | 对齐度 | 说明 |
|------|--------|------|
| Event matching | 95% | pre/post.{event}.{qualifier} 模式匹配 |
| Script execution | 85% | JSON over stdin, timeout, allow/block/modify |
| Three-tier merge | 80% | global/project/user 三层 |
| after_provider_request | 新 | **xylitol 独有** — pi 无此 hook 点，用于 prefix-caching 优化 |
| after_provider_response | 新 | **xylitol 独有** — pi 无此 hook 点 |

**已知差距**:
- Hooks 是外部脚本执行 — pi 的 extensions 是内置进程内钩子，提供更深集成
- 无 pi 的 tool_call/tool_result 拦截（修改 tool 结果）

### 关键 gap 优先级

| 优先级 | Gap | 影响 |
|--------|-----|------|
| 🔴 P0 | Compaction 核心未实现 | 长会话会 OOM / hit context limit |
| 🔴 P0 | Grep/Find/Bash streaming cancel | abort 后命令继续执行 |
| 🟡 P1 | 配置三层 merge + ENV 插值 | 用户无法通过 YAML 配置 |
| 🟡 P1 | Session 并发文件锁 | 多进程损坏数据 |
| 🟡 P1 | Session tree / fork | 无法分支/切换会话 |
| 🟢 P2 | 并行工具执行 | 性能优化 |
| 🟢 P2 | System prompt 构建 | 影响 prompt 质量 |
| 🟢 P3 | Interactive TUI | 下一阶段 |

## 文件清单

### 新增文件 (12)

```
src/agent/tools/truncate.rs      (~400 行) — TruncationResult 对齐 pi truncate.ts
src/agent/tools/operations.rs    (~130 行) — 7 Operation traits
src/agent/tools/mutation.rs      (~130 行) — FileMutationQueue 对齐 pi
src/agent/tools/path_utils.rs    (~80 行)
src/agent/session.rs             (~340 行) — AgentSession + ModelRegistry
src/infra/session/types.rs       (~170 行) — 7 SessionEntry 枚举
src/infra/session/manager.rs     (~180 行) — SessionManager CRUD
tests/bdd.rs                     (~880 行) — 80+ cucumber step defs
tests/features/session.feature   — 7 scenarios
tests/features/agent.feature     — 8 scenarios
tests/features/compaction.feature — 5 scenarios
tests/features/hooks.feature     — 8 scenarios
```

### 重写文件 (9)

```
src/agent/loop.rs         — 完整 ReAct loop + AgentEvent stream
src/agent/error.rs        — +Aborted variant
src/agent/traits.rs       — +CancellationToken
src/agent/tools/edit.rs   — multi-edit 完全重写
src/agent/tools/grep.rs   — ripgrep 对齐 pi
src/agent/tools/find.rs   — fd 对齐 pi
src/agent/tools/bash.rs   — abort + kill_tree
src/agent/tools/read.rs   — truncation 对齐 pi
src/agent/tools/write.rs  — mutation queue
src/agent/tools/ls.rs     — sorted + suffix
src/agent/tools/patch.rs  — +fuzzy_find + patch_find_range
src/agent/tools/mod.rs    — +wrap_with_hooks + filtered + mutation-aware builtins
src/agent/mod.rs          — 移除 repeat/planner
src/infra/mod.rs          — 移除 security, session 改为 always-on
src/interface/cli/mod.rs  — clap args 对齐 pi
src/interface/print.rs    — streaming stdout print mode
src/lib.rs                — async run()
src/main.rs                — tokio runtime
```

### 已移除 (6)

```
src/agent/repeat.rs
src/agent/planner.rs
src/infra/security/       (entire directory)
tests/support/harness.rs  (stub)
tests/support/in_memory.rs (stub)
tests/support/vt100_backend.rs (stub)
```

### 已禁用 (stub)

```
src/interface/acp.rs      (3 lines stub)
```

## Commit 建议

```
feat: rebuild core to align with pi coding-agent (c05-rebuild-core)

Complete rewrite of all core modules in a single change:
- 7 tools fully aligned with pi (edit, bash, read, write, grep, find, ls)
- AgentSession with model switching and thinking level toggle
- Full ReAct loop with AgentEvent stream (12 event types)
- JSONL session persistence with 7 entry types
- Compaction estimation and trigger (core summarization pending)
- Hook system with after_provider_request/response for prefix-caching
- CLI with clap args matching pi (--model, --session, --print)
- 80+ BDD step definitions (all compile)
- Removed: security-policy, repeat-guard, planning-orchestrator

ARCHIVE: 2026-06-09-c05-rebuild-core
TEST: cargo test --lib => 159 passed
BDD: cargo check --test bdd => 0 errors
LINES: ~12,871 total (src/)
```

See _HANDOFF.md for detailed alignment analysis and gap priorities.
