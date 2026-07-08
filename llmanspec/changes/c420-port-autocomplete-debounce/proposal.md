---
change_id: c420-port-autocomplete-debounce
title: "port autocomplete debounce — async+CancellationToken+fd递归+debounce 补齐（pi autocomplete.ts → Rust）"
status: draft
priority: 420
depends_on: []
author: agent
---

# c420-port-autocomplete-debounce

## Why

`autocomplete.ts`（pi 786 行 → xy 534 行，-32%）缺三个关键能力：async（pi 用 `Promise`，xy 全同步）、`walkDirectoryWithFd`（pi 用 fd 子进程递归搜索，xy 只有无递归的 `read_dir` 桩）、AbortController/Debounce（pi 用 `AbortSignal` + `setTimeout 250ms` 取消在途查询，xy 无）。没有这三者，editor 集成时无法取消过时查询，每次按键都触发同步 `read_dir` 会卡主线程。

按 _HANDOFF §二 依赖序，autocomplete（6.3）在 editor（6.4）之前。editor 移植时需消费已 rebounce 的 autocomplete。

### 证据

pi `autocomplete.ts`:
- `getSuggestions` 返回 `Promise<AutocompleteSuggestions | null>`，接受 `{ signal: AbortSignal }`（:191-196）
- `getFuzzyFileSuggestions` 调用 `walkDirectoryWithFd`（:560-614），`walkDirectoryWithFd` spawn fd 子进程（:152-254），约 200 行
- editor.ts 负责 debounce 循环：`setTimeout 250ms` + `AbortController.abort()`（pi editor.ts，不在 autocomplete.ts）

xy `autocomplete.rs`:
- `type Awaitable<T> = T`（同步别名，无 CancellationToken）
- `get_fuzzy_file_suggestions` 掉到 `read_dir` 桩（限单层目录，无递归、无 fd）
- 零 debounce/取消机制

### 往期验证

- c405 第 3 层 `#[tokio::test(start_paused)]` 已验证可用于确定性 debounce 测试
- c415 `Instant` 参数注入已验证时间可测
- tokio `CancellationToken` 是 Rust 生态的 AbortController 标准等价物

## What Changes

1. **Async trait + CancellationToken**：
   - `AutocompleteProvider` trait 增加 `async_get_suggestions` 方法（接受 `CancellationToken`）
   - `CombinedAutocompleteProvider` 实现 `async_get_suggestions`，在 fd 子进程 io 循环中检查 `CancellationToken`

2. **walk_directory_with_fd**：
   - 新增 `packages/xylitol-tui/src/autocomplete_fd.rs`：`walk_directory_with_fd()` 函数
   - spawn `fd` 子进程（`std::process::Command`），对齐 pi 的 `--base-directory`/`--max-results`/`--type f`/`--type d`/`--follow`/`--hidden` 参数
   - 支持 `CancellationToken` 取消 → 发送 SIGKILL
   - `build_fd_path_query()` 构建 fd 的 `--full-path` 正则查询

3. **DebouncedAutocomplete wrapper**：
   - 新增 `DebouncedAutocomplete<P>` wrapper struct（在 `autocomplete.rs`）
   - 包装任意 `AutocompleteProvider`，提供 `debounced_get_suggestions(delay_ms, lines, cursor, ct)` 方法
   - 内部用 `tokio::time::sleep` + `CancellationToken` 实现 250ms debounce
   - 新调用到达时自动取消旧的 `CancellationToken`

4. **lib.rs 导出** `walk_directory_with_fd`、`DebouncedAutocomplete`

5. **测试（c405 第 1+3+4 层）**：
   - 第 1 层：`walk_directory_with_fd` 在 tempdir 的递归搜索
   - 第 3 层：debounce 窗口（250ms 内多次调用只触发最后一次）、CancellationToken 取消在途查询
   - 第 4 层（proptest）：补全列表一致性（fd 结果 vs read_dir 结果的一致性不变量）

## Capabilities

- `autocomplete`（修改：MODIFY autocomplete spec）

## Impact

- `packages/xylitol-tui/src/autocomplete.rs`（修改，~100 行）
- `packages/xylitol-tui/src/autocomplete_fd.rs`（新建，~120 行）
- `packages/xylitol-tui/src/lib.rs`（导出）
- `packages/xylitol-tui/tests/autocomplete_test.rs`（新建，~15 测试）
- `llmanspec/specs/autocomplete/spec.toon`（不存在则新建，否则 delta）

## Non-goals

- **不在 editor 中集成 autocomplete**——那是 c430（editor-autocomplete）的范围
- **不移植 `native-modifiers.ts`**——macOS 专属
- **不在 xy 中引入 fd 二进制依赖**——fd 是可选的运行时依赖（`fdPath: Option<String>`），无 fd 时回退到非递归 read_dir
