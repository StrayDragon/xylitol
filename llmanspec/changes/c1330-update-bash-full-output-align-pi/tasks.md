# Tasks: c1330

## Specs

- [x] live `infra-bash`：收紧 be3（footer + path）
- [x] live `agent-tools`：bash 工具截断 JSON MUST NOT 含全量 stdout/stderr
- [x] live `app-tui-transcript`：Full output 行 warning 色
- [x] attach change；feature 场景（可复用/增补 bash-truncate）

## Implement

- [x] `OutputSnapshot::display_content` → pi footer；单测
- [x] `bash_exec` / `RealBashOperations`：`output` 用 display_content
- [x] `BashTool` 非流式 + 流式：截断 JSON，无全量 stdout/stderr
- [x] TUI scrollback：Bash/Tool 输出 footer 行 warning paint
- [x] `bash_output_body`：保留 Full output 行，勿仅用 `(truncated)` 覆盖信息

## Verify

- [x] `cargo test -p xylitol --lib infra::tools::accumulator`
- [x] 相关 BDD / `just qa` 子集
