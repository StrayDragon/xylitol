# Design — TUI @ 路径补全接入 fd

## 目标

在大仓库里让 `@` 路径补全用 fd 召回深层文件/目录；fd 不可用或无结果时静默回落现有单层扫描；不改触发符与编辑语义。

## 现状（代码事实）

- 产品 `AtPathSource`：`crates/xylitol-tui/src/completion.rs:382`；`get_fuzzy_file_suggestions` 用 `std::fs::read_dir`（`:416`），`.take(20)`（`:442`）。
- 引擎 fd：`walk_directory_with_fd`（`crates/xylitol-tui/src/autocomplete_fd.rs:48`）；`CombinedAutocompleteProvider::new_with_fd`（`autocomplete.rs:78`）；fd 评分/映射在 `autocomplete.rs:297-332`。
- 产品 completion sources 装配：`crates/xylitol-app-tui/src/layout/root/mod.rs:241-286`；`AtPathSource::new(self.at_path_base.clone())`（`:283`）。
- 无 `which` crate；仓库内 `which` helper 在 `plugins/xylitol-official-plugin-tools/src/process/shell.rs:90`（插件内私有）。
- fd 二进制名：`fd`（macOS / Arch）或 `fdfind`（Debian/Ubuntu）。

## 引擎 helper

新增公共函数（放 `completion.rs` 或 `autocomplete_fd.rs`）：

```rust
pub fn fd_path_suggestions(
    base_path: &Path,
    fd_path: &str,
    query: &str,
    is_at: bool,
    is_quoted: bool,
    max_results: usize,
) -> Vec<AutocompleteItem>
```

- 内部：`parse_path_prefix` → `walk_directory_with_fd(base, fd, raw, 100, ct)` → 评分（复用 `score_entry` 逻辑，可提为公共）→ 取前 20 → `path_autocomplete_item`。
- fd 空结果 → 返回空，由产品回落 read_dir。
- 取消：新建 `CancellationToken::new()`；产品同步调用，暂不接取消（后续）。
- 单测：无 fd / 有 fd / 深层路径 / 目录后缀 / quoted。

## 产品接线

- `AtPathSource` 增加 `fd_path: Option<String>`；`new(base)` 保持；`new_with_fd(base, fd)` 新增。
- 启动时 `find_fd()`：遍历 `PATH`，找 `fd` 或 `fdfind` 可执行文件；找不到 `None`。
- `install_completion_sources`：`AtPathSource::new_with_fd`（找到 fd）或 `new`（回退）。
- `suggestions`：若 `fd_path` 有值，先 `fd_path_suggestions`；结果非空返回；否则 `get_fuzzy_file_suggestions` 回落。
- 不改 `probe` / `should_dismiss` / `extract_at_prefix`。
- 目录项显示后缀 `/`；quoted 语义沿用 `path_autocomplete_item`。

## 配置 / 可观测

- 不加配置；fd 可用即用，不可用回落。
- 可在 debug log 打一条 `fd_path` 选择结果（不写 stdout/stderr，走 `xylitol::tui` log）。
- 不做用户面开关（后续若需要再加）。

## 实施任务

1. 引擎：提公共 `score_entry` / `fd_path_suggestions`；单测。
2. 产品：`AtPathSource` fd 字段 + `new_with_fd`；`find_fd()`。
3. 接线：`install_completion_sources` 选择 fd / fallback。
4. 补全：fd 结果非空用 fd；空则 read_dir。
5. 测试：大仓库深层路径召回；fd 不存在回落；目录/文件/quoted；CJK 文件名。
6. specs：`app-tui-input` 新规则；`package-tui-*` helper（若需要）。

## 验证

- 单测：fd helper 评分 / 目录后缀 / quoted；无 fd 回退。
- 产品 harness：`@src/uti` 在大仓库召回 `src/utils.rs`；`@` 空查询列出顶层；fd 不存在时行为与现状一致。
- 真机 tmux：大仓库 `@` 补全；fd 安装/卸载两种。
- 回归：`just test-tui`、相关 harness、`just qa`。

## 风险 / 待确认

- fd 同步调用可能阻塞 UI；现有引擎实现也是同步，大仓库可接受；后续可接 CancellableToken / 异步。
- fd 二进制名因发行版不同；`fd` / `fdfind` 都找。
- fd 的 `--hidden` / `.git` 排除沿用引擎参数；产品不另设。
- 召回质量依赖 fd 查询语法；路径分隔符用 `build_fd_path_query`。
- 不做缓存；每次补全 spawn fd；高频输入可能多次 spawn，后续可加 debounce / cache。
