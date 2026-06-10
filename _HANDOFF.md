# Handoff: c07-fix-bdd-scenarios + 累计状态

> 最后更新：2026-06-10 · c05/c06 已归档, c07-fix-bdd-scenarios 进行中
> 上游参考：`../pi-mono` (pi coding-agent 源码)

## 当前 BDD 状态

```bash
cargo test --test bdd -- --test-threads=1
# → 77 passed, 0 failed ✅
```

| 指标 | 数值 |
|------|------|
| tests/bdd.rs | ~1450 行 |
| 步骤定义 | ~175 个 `#[given]`/`#[when]`/`#[then]` (全部 typed placeholder) |
| 场景绑定 | 77 个 `#[scenario(path, name)]` 跨 12 个 feature 文件 |
| Fixture | 3 个独立类型 Workspace / SessionStore / AgentState |
| BDD 框架 | **rstest-bdd** (c06 从 cucumber-rs 迁移, c07 修完所有 gap) |
| 通过率 | **77 / 77 (100%)** |

## 累计变更历史

| # | 变更 | 状态 | 说明 |
|---|------|------|------|
| c05 | rebuild-core | ✅ 已归档 | 完整复刻 pi 核心: 7 tools, ReAct loop, session, hooks, CLI |
| c06 | update-bdd-framework | ✅ 已归档 | cucumber-rs → rstest-bdd 迁移完成 |
| c07 | fix-bdd-scenarios | 🔄 当前 | 修复 36 个 BDD 场景失败, 77/77 全绿 |

## c07 修复总结

### 根因 (6 类, 36 个失败)

| 类别 | # | 根因 | 修复方式 |
|------|---|------|----------|
| SessionEntry type 冲突 | 7 | `EntryBase.entry_type` (`#[serde(rename="type")]`) 与 enum `#[serde(tag="type")]` 产生重复 key | `#[serde(skip,default)]` + 手动构建 JSON |
| `{text}` 引号捕获 | 14 | rstest-bdd `{text}` 捕获包含步骤文字中的引号 | `strip_quotes()` helper |
| OR 子句未拆分 | 6 | `"A" 或 "B"` 被当作整体字符串 | `check_or_contains()` 按 `或` 拆分 |
| 缺失步骤 | 3 | `列出所有会话`/`结果列出 {entry}` 等未注册 | 新增步骤定义 |
| Bash JSON 格式 | 3 | then 步骤用 `contains` 检查整个 JSON 字符串 | 解析 JSON 提取 stdout/combined 字段 |
| Feature 文件适配 | 3 | DataTable/OR clause 等 rstest-bdd 行为差异 | 修改 feature 文件适配 |

### 修改的文件

```
src/infra/session/types.rs       — EntryBase/SessionHeader type 字段修复
src/infra/session/manager.rs     — create() 手动构建 header JSON
tests/bdd.rs                     — 步骤定义修复 (~200 行变更)
tests/features/{read,write,edit,bash,grep,find,ls,session}.feature — 适配
```

## 模块对齐度分析 (更新)

基于 c07 完成后的实际运行结果：

### 已确认无 gap

| 模块 | 状态 | 说明 |
|------|------|------|
| **7 built-in tools** | ✅ | read/write/edit/bash/grep/find/ls — 全部 BDD 通过 |
| **Session persistence** | ✅ | create/append/load/list — JSONL + 类型安全 |
| **Hooks** | ✅ | pre/post dispatch + block/modify/allow |
| **Compaction** | ✅ | Context usage + shouldCompact + threshold |
| **Agent loop** | ✅ | Turn events + thinking + model switching |

### 剩余 gap (从 c05 追溯)

| 优先级 | Gap | 说明 |
|--------|-----|------|
| 🔴 P0 | **Compaction 核心未实现** | LLM-based summarization (stub only) |
| 🔴 P0 | **Grep/Find/Bash streaming cancel** | 当前用 tokio spawn + timeout; pi 用 streaming |
| 🟡 P1 | **配置三层 merge + ENV 插值** | 当前仅 global tier |
| 🟡 P1 | **Session 并发文件锁** | flock 未实现 |
| 🟡 P1 | **Session tree / fork** | stub only |
| 🟢 P2 | **并行工具执行** | 当前串行 |
| 🟢 P2 | **System prompt 构建** | 加载 context files + skill prompts |
| 🟢 P3 | **交互式 TUI** | 已移除 (→ zirvox) |

## cargo test 使用方式

```bash
cargo test --test bdd                          # 全部 77 个场景
cargo test test_read_entire_file               # 精确运行单个场景
cargo test read                                # 按名称过滤
cargo test --test bdd -- --test-threads=1      # 串行执行
cargo test --test bdd -- --nocapture           # 看完整输出
```

## 文件结构概览

```
src/
  agent/          — agent loop, tools, config, prompts, session
  infra/          — hooks, security, skills, session, config
  interface/      — CLI/RPC and user-facing output
tests/
  bdd.rs          — BDD 步骤定义 (~1450 行, 77 个场景)
  features/       — 12 个 .feature 文件
llmanspec/
  changes/        — c06 (已归档), c07 (当前)
  specs/          — 模块能力规范
docs/
  testing.md      — 测试层级指南
  adk-rust-dependency-assessment.md — 历史评估 (adk-rust 已移除)
```
