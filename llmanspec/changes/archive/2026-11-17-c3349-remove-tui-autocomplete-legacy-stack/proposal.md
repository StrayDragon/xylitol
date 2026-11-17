---
depends_on: []
needs_specs_change: false
title: 收敛 TUI 补全到 CompletionSource 单栈并修订补全检索合约
branch: sdd/c3349-remove-tui-autocomplete-legacy-stack
base_branch: main
base_sha: 79d57e9e3a0bd14b958897ae4276d65cc6f34a85
---

# 收敛 TUI 补全到 CompletionSource 单栈

## Why

`xylitol-tui` 补全存在两套平行实现：旧栈 `AutocompleteProvider` / `CombinedAutocompleteProvider` / `DebouncedAutocomplete`（src/autocomplete.rs，约 385 行）全仓非测试消费者为 0，生产消费全部由新栈 `CompletionSource`（src/completion.rs）承载；fd 建议评分在两栈各有一份。架构审查（2026-11-17）删除验证判定旧栈删除后复杂度不回来，仅词汇表与帮助函数仍住在旧栈文件被新栈反向 import。合约侧 `package-tui-autocomplete` 三条规则中两条的主语类型将被删除（CombinedAutocompleteProvider / DebouncedAutocomplete）；`package-tui-editor-autocomplete` r1608 仍钉住代码中已不存在的 `set_autocomplete_provider` 兼容 shim——合约与代码已漂移。

## What Changes

- 删除旧栈三件：`AutocompleteProvider` trait、`CombinedAutocompleteProvider`（含旧 fd 评分副本）、`DebouncedAutocomplete`。
- 词汇表与帮助函数迁入 completion.rs：`AutocompleteItem` / `SlashCommand` / `AutocompleteSuggestions` / `extract_at_prefix` / `extract_dollar_prefix` / `parse_path_prefix` / `expand_home_path` / `path_autocomplete_item`；lib.rs re-export 公开名单不变。
- `tests/autocomplete_test.rs` 拆解：旧栈构造用例删除；`build_fd_path_query` / fd 递归搜索 / 取消 SIGKILL 用例保留。
- specs 修订（包实例，条款处置见 design.md）：
  - `package-tui-autocomplete` 重新划界为「补全 fd 检索与取消」：r1578 删除（活语义并入 r1579），r1579 改述（主语去类型化、并入 CancellationToken/SIGKILL 契约），r1580 删除（防抖需求由 editor-autocomplete r1611 承载）。
  - `package-tui-editor-autocomplete` r1608 删除「set_autocomplete_provider 兼容 shim」句。
- 无破坏性升级路径需求：旧栈类型无外部消费者（含 examples 与 app-tui），公开 API 名单不变。

## Capabilities

- `package-tui-autocomplete`（crates/xylitol-tui/llmanspec/specs/，包实例）——重新划界与条款修订
- `package-tui-editor-autocomplete`（同上）——r1608 shim 句清理

## Impact

- 代码：crates/xylitol-tui（src/autocomplete.rs 大部分删除、src/completion.rs 扩容、src/lib.rs re-export 路径、editor 与测试 import）。
- 产品行为不变：CompletionSource 路径的补全行为、fd 回落、popup 交互均不变。
- 根 specs 无落地：frontmatter `needs_specs_change: false`；包实例 live feature 在绑定分支改并提交（包实例例外见 crates/xylitol-tui/llmanspec/AGENTS.md）。
- 关联 ADR：c3145 的公共落点决策（fd_path_suggestions 于 completion.rs）与本 change 收敛方向一致，无需重开；旧栈 fd 评分为其未清遗留。
