# c376 Design

> spike 验证：未闭合代码块也高亮（vendored parser 容错），所以行差量 commit 可行。

## 机制

每次 TextDelta：
1. `finalized.push_str(delta)`
2. `all_lines = render_markdown(&finalized, width, style)` 全量重渲
3. `stable_end = finalized.rfind('\n').map(|i| i+1).unwrap_or(0)` 稳定边界（最后换行）
4. `stable_lines = render_markdown(&finalized[..stable_end], width, style)` 重渲稳定前缀
5. `new_committed = stable_lines.len()`
6. 若 `new_committed > committed_count`：commit `all_lines[committed_count..new_committed]`（新增稳定行，带高亮）
7. `committed_count = new_committed`
8. mutable 区显示 `all_lines[committed_count..]`（尾部，带高亮）

TurnEnd：commit `all_lines[committed_count..]`（剩余全部），清空。

## spike 证据

未闭合 ` ```rs\nfn a() {\n    let x = 1;\n` 渲染 2 行均 colored=true。闭合后多 1 行（`}`），前 2 行一致。行差量 commit 完美适配。

## 不在本变更范围

表格 holdback、source-backed cell reflow、CustomTerminal。
