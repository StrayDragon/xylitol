# Design — TUI 工具参数折叠层与用户长消息折叠

## 目标

工具参数与用户长消息都变成「默认缩略、可点击展开」的端侧折叠；header 不再被长参数撑高；长用户消息不再独占整屏；复用现有 expandable output / fold hit / cache。

## 现状（代码事实）

- 工具 header：`paint_tool_header_line`（`crates/xylitol-app-tui/src/widgets/scrollback/paint.rs:64`）用 `args_preview` + `push_wrapped`（`:150`）；产品路径 `human_tool_args_preview_with_path(..., usize::MAX)`（`bridge/mod.rs:80` / `:98`），不截字符。
- `UiEntry::Tool` 字段：`args_preview` / `tool_path` / `write_content` / `display_diff` / `output`（`bridge/model.rs:107-...`）；无 `args_full`。
- `upsert_tool_entry` 从 `ToolExecutionStart.args` 构造（`bridge/mod.rs:40`）；历史重建在 `bridge/session_tree.rs:470`。
- 工具块已有 write 正文 10 行、output 5 行、diff 12 行 viewport（`widgets/scrollback/diff.rs:23-27`）；args 没有。
- MCP args 进 output：`mcp_tool_body`（`tool_display.rs:37`）pretty JSON 无上限。
- 用户消息：`paint_user_block`（`paint.rs:364`）全文 `push_wrapped`；无折叠目标；`UiEntry::User { text }`（`bridge/model.rs`）。
- `FoldTarget` / `FoldHitTable` 在 `widgets/fold_hit.rs`；`ScrollbackFold` 在 `widgets/scrollback/mod.rs:40`。
- `render_expandable_output`（`crates/xylitol-tui/src/components/expandable_output.rs:29`）支持 preview / full / hint / viewport。

## 工具参数层

### 数据

- `UiEntry::Tool.args_full: Option<String>`：
  - bash：`args["command"]`。
  - MCP（`is_mcp_tool_name`）：`pretty_json_value(args)`。
  - 其他：`None`（header 已有路径/摘要；避免重复）。
- 历史重建：ToolCall part 的 `arguments` 同规则填充。
- 不把 args_full 放进 `project_for_llm`；只做端侧展示。

### Header

- 单行：`▾ Bash $ <command 首行>`；`truncate_to_width` 到块宽，超宽 `…`；不 `push_wrapped`。
- MCP：只画工具名；文件族：路径 + 行域；write：路径。
- 保留折叠三角；header 点击切换块折叠，与现状一致。

### Args viewport

- 默认 `tool_args` 模式：hidden（summary）/ preview（recent）/ full（full）。
- preview：5 行 head（命令/JSON 开头更有信息量），`... (N more lines, click to expand)`。
- 展开：全文；沿用 `render_expandable_output`，不新造组件。
- 命中：`FoldTarget::ToolArgs(id)`；hint 行与整块可点。
- 缓存：`entry_fingerprint` 纳入 args 展开态；args_full 内容变化时失效。

## 用户长消息折叠

- 模式：`user_message` = preview（默认 3 视觉行）| full。
- preview：先按宽度 `push_wrapped` 全文，取前 3 视觉行；若总行数 > 3，追加 `... (N more lines, click to expand)`；点击展开全文。
- full：现状全文。
- 命中：`FoldTarget::User(key)`；key = `user-{index}-{hash(text)}`，resume/travel 稳定；重复文本用 index 区分。
- 缓存：fingerprint 纳入展开态。
- 复制：展开后可全选；折叠时 hint 不参与复制语义（选中行仍是文本）。

## 配置

```yaml
tui:
  activity_fold:
    visible_items:
      tool_args: hidden        # hidden | preview | full
      user_message: preview    # preview | full
    user_message_preview_lines: 3
```

- 默认：tool_args=hidden；user_message=preview；preview_lines=3。
- 显式覆盖优先于预设；运行时点击写逐块覆盖。
- 非法值失败；preview_lines 1–10。

## 渲染改造

- `paint_tool_block`：header 单行；若 args 模式非 hidden，渲染 args viewport；output/diff 仍按 c3120 模式。
- `paint_user_block`：增加 `fold` / `expanded` 参数；preview 截断 + hint；full 全文。
- `ScrollbackFold`：增加 `tool_args_mode` / `user_message_mode` / `user_preview_lines`；per-id / per-user overrides。
- `FoldHitTable`：新增 `ToolArgs` / `User` 目标；命中行由渲染返回。
- `entry_fingerprint`：纳入 args/user 展开态与 args_full hash。

## Spec 变更点

- 新增规则：工具参数层（header 单行摘要 + args viewport + 点击）。
- 新增规则：用户长消息 preview + 点击展开。
- r1343：保持 header 摘要不得携带完整 args JSON；args 层是正文，不是 header。
- r1344/r1346/r1347：write/output/diff 预览行数不变；args 层不改变这些 cap。
- `runtime-config`：新增两个 visible_items 字段。

## 实施任务

1. bridge：`UiEntry::Tool.args_full`；`upsert_tool_entry` / 历史重建填充；单测。
2. header：`paint_tool_header_line` 单行截断；bash/MCP/文件族分支；snapshot。
3. args viewport：`FoldTarget::ToolArgs`；preview/full/hidden；命中；cache。
4. user fold：`FoldTarget::User`；preview 3 行 + hint；展开；cache；snapshot。
5. config：`tool_args` / `user_message` / `preview_lines`；默认与校验。
6. 点击/键位：沿用折叠命中表；不新增默认和弦。
7. 测试：长 bash header 不撑高；args 展开；80 行用户消息 3 行预览；点击展开；历史重建；CJK 宽字符。
8. specs：新规则与场景；BDD binding。

## 验证

- snapshot：长 bash 命令 header 1 行；args 收起/展开；MCP args JSON 5 行 preview；80 行用户消息 3 行 + hint。
- harness：点击 hint 展开；再次收起；resume 重建后展开态默认收起。
- 缓存：args/user 展开态变化触发重绘；未变化不重绘。
- 真机 tmux：提交 80 行；展开/收起；bash 长命令；MCP 工具。

## 研究

- tmux 长输入 / 工具参数观测：[`research/tmux-long-input-and-tool-args.md`](./research/tmux-long-input-and-tool-args.md)。

## 风险 / 待确认

- 用户消息 preview 行数：3 行是默认；用户可配置 1–10。是否所有等级都 preview 还是 full 档全文，由 c3120 的 visible_items 决定。
- args_full 可能含敏感内容（命令 / JSON）；只端侧展示，不进模型上下文；复制由用户主动。
- header 单行截断可能丢失命令关键尾部；args 层可展开，且 hint 明示。
- 历史重建的 args_full 依赖 ToolCall arguments 完整；流式未齐时可能为空，结束后补齐。
- 与 c3120 的 visible_items 结构耦合；c3120 先落地容器字段，c3125 扩展两项。
