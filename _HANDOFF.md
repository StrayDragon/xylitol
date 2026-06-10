# Handoff: c06-update-bdd-framework + 累计状态

> 最后更新：2026-06-10 · 已归档 c05-rebuild-core → 当前变更 c06-update-bdd-framework
> 上游参考：`../pi-mono` (pi coding-agent 源码)

## 累计变更历史

| # | 变更 | 状态 | 说明 |
|---|------|------|------|
| c05 | rebuild-core | ✅ 已归档 | 完整复刻 pi 核心: 7 tools, ReAct loop, session, hooks, CLI |
| c06 | update-bdd-framework | ✅ 当前 | cucumber-rs → rstest-bdd 迁移完成 |

## c06 关键发现：cucumber-rs 从未运行过

cucumber-rs 需要 `[[test]] name = "bdd" harness = false` 才能运行自定义 `async fn main()` binary。
这个配置项**从未出现在 Cargo.toml 中**，因此 `cargo test --test bdd` 自始至终运行了 **0 个测试**。

```
cucumber-rs (迁移前):  cargo test --test bdd  →  0 passed, 0 failed  (harness=false 未配置)
rstest-bdd (迁移后):    cargo test --test bdd  →  41 passed, 36 failed
```

rstest-bdd 不依赖 custom harness — 每个场景是一个普通 `#[scenario] fn`，原生 `cargo test` 即可运行。

## 当前 BDD 状态

| 指标 | 数值 |
|------|------|
| tests/bdd.rs | 1290 行 |
| 步骤定义 | 171 个 `#[given]`/`#[when]`/`#[then]` (全部 typed placeholder `{name:type}`) |
| 场景绑定 | 77 个 `#[scenario(path, name)]` 跨 12 个 feature 文件 |
| Fixture | 3 个独立类型 Workspace / SessionStore / AgentState (vs 原本 1 个 World) |
| 通过 | **41 / 77** (首次有测试真正执行) |
| 失败 | 36 个 — 全部为预存在 gap, 非框架迁移导致 |

### 失败分类

| 类别 | 数量 | 根因 | 影响场景 |
|------|------|------|----------|
| Session manager 未初始化 | 7 | session.feature 中多个场景的 Background 缺少 `会话存储目录已初始化` | session_list, session_fork, session_model_change, session_thinking_change, session_jsonl_format, session_tree_nav, agent_auto_save |
| 工具步骤变体缺失 | 16 | read/write/bash/grep/find/ls 的 limit/offset/不区分大小写/字面量/不传参 等变体步骤未实现 | read_nonexistent, read_out_of_bounds, bash_simple, bash_stderr, bash_merge, grep_literal, grep_no_match, find_basic, find_no_match, find_recursive, find_absolute_rejected, find_limit?, ls_default_path, ls_file_not_dir, ls_with_files, ls_limit? |
| 工具输出格式差异 | 10 | write 输出带引号包裹, edit 错误文本不完全匹配, BOM/CRLF/unicode 边缘情况 | write_new_file, write_overwrite, write_create_parents, write_byte_count, edit_single_replace, edit_bom, edit_unicode, edit_preserves_crlf, edit_returns_diff, edit_empty_oldtext |
| Edit 多编辑器 + DataTable | 2 | `进行N处替换:` 解析 + `非唯一的 oldText`/`无变更的编辑` 错误消息匹配 | edit_multi_replace, edit_nonunique_rejected, edit_noop_rejected, edit_overlap_rejected |
| Hook block | 1 | Hook block 逻辑边缘情况 | hook_block |

### cargo test 使用方式

```bash
cargo test --test bdd                          # 全部 77 个场景
cargo test test_read_entire_file               # 精确运行单个场景
cargo test read                                # 按名称过滤
cargo test --test bdd -- --test-threads=1      # 串行执行
cargo test --test bdd -- --nocapture           # 看完整输出
```

## 模块对齐度分析 (更新)

基于 `c06` 的实际运行结果，以下更新之前的 gap 分析：

### 确认的 gap (在 cucumber-rs 中同样未实现，现在暴露出来了)

| 模块 | Gap | 严重度 | 说明 |
|------|-----|--------|------|
| Session | 多个场景缺 Background | 🔴 P0 | 会话列表/分叉/模型切换/思考切换/JSONL 格式的测试无法运行 |
| Tools | write 输出格式 | 🟡 P1 | write 工具在 `{content:string}` placeholder 解析后输出引号包裹 |
| Tools | edit 错误消息匹配 | 🟡 P1 | edit 错误消息中的 unicode/identical/overlap 文本不完全匹配 feature 预期 |
| Tools | read/batch/find/grep/ls 变体 | 🟡 P1 | limit/offset/不区分大小写/字面量/无参 等变体步骤未实现 |
| Tools | bash stdout/stderr | 🟡 P1 | bash 输出 JSON 结构与 feature 断言不匹配 |
| Edit | DataTable 解析 | 🟡 P1 | `进行2处替换:` 的 DataTable 步骤未处理 |
| Agent | agent_auto_save session mgr 未初始化 | 🟡 P1 | Background 缺 `会话存储目录已初始化` |

### 累计 gap 优先级 (更新)

| 优先级 | Gap | 来源 | 下一步 |
|--------|-----|------|--------|
| 🔴 P0 | **Session manager 在所有场景中自动初始化** | c06 运行暴露, c05 预存 | 在 rstest-bdd fixture 中自动初始化 session dir |
| 🔴 P0 | **Compaction 核心未实现** | c05 | 参考 `../pi-mono` 实现 LLM-based summarization |
| 🔴 P0 | **Grep/Find/Bash streaming cancel** | c05 | 参考 `../pi-mono` 改用 streaming output |
| 🟡 P1 | **Write 工具输出格式 + Edit 错误消息对齐** | c06 运行暴露 | 逐场景修复 BDD 步骤断言 |
| 🟡 P1 | **缺失的工具步骤变体** | c06 运行暴露 | 补全 read/write/bash/grep/find/ls 的 offset/limit/不区分大小写/etc 变体 |
| 🟡 P1 | **配置三层 merge + ENV 插值** | c05 | 参考 `../pi-mono/src/infra/config/` |
| 🟡 P1 | **Session 并发文件锁** | c05 | 参考 `../pi-mono` 的 flock |
| 🟡 P1 | **Session tree / fork** | c05 | 参考 `../pi-mono/src/infra/session/` |
| 🟢 P2 | **并行工具执行** | c05 | 参考 `../pi-mono/src/agent/loop/` |
| 🟢 P2 | **System prompt 构建** | c05 | 加载 context files, 拼接 skill prompts |
| 🟢 P3 | **Interactive TUI** | c05 | 参考 `../pi-mono/src/interface/tui/` |

## 文件变更汇总

### c06 变更

| 操作 | 文件 |
|------|------|
| 修改 | `Cargo.toml` — cucumber→rstest-bdd 替换 |
| 重写 | `tests/bdd.rs` — 1191→1290 行, World→fixture, regex→placeholder |
| 新增 | `llmanspec/changes/c06-update-bdd-framework/` — proposal/design/tasks/delta |

### 累计文件清单 (c05 + c06)

```
src/agent/tools/truncate.rs        (~400 行)
src/agent/tools/operations.rs      (~130 行)
src/agent/tools/mutation.rs        (~130 行)
src/agent/tools/path_utils.rs      (~80 行)
src/agent/session.rs               (~340 行)
src/infra/session/types.rs         (~170 行)
src/infra/session/manager.rs       (~180 行)

重写:
src/agent/loop.rs, error.rs, traits.rs
src/agent/tools/{edit,grep,find,bash,read,write,ls,patch,mod}.rs
src/interface/cli/mod.rs, print.rs
src/lib.rs, main.rs

测试:
tests/bdd.rs                       (~1290 行) — rstest-bdd, 171 steps, 77 scenarios
tests/features/{session,agent,compaction,hooks}.feature + 8 原有
```

## 下一步 (c07-fix-bdd-scenarios)

1. **P0: 修复 session manager 自动初始化** — 在 fixture 中自动初始化 session dir
2. **P1: 补全工具步骤变体** — read offset-only, write no-content, bash no-cmd, grep/find/ls variant steps
3. **P1: 对齐 write/edit 输出格式** — 修复断言与工具实际输出的差异
4. **验证目标**: `cargo test --test bdd -- --test-threads=1` → 77/77 通过

然后将基于 `../pi-mono` 源码逐个修复 c05 中记录的中等优先级 gap：
config merge, session flock, session fork, streaming cancel, compaction core, 并行工具执行。
