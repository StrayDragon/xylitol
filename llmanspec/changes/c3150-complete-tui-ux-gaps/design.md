# Design — 补齐 TUI UX 验收缺口

## 目标

逐项对齐 handoff 6.2 验收，不新增产品面开关，不改变已归档 change 的主语义。

## 缺口与实现

### c3120

- `ScrollbackFold` 增加 thinking 单行最新文本 / 全文语义：summary/quiet 收起时 header 追加最新一行；recent/full 正文全文。
- `UiRoot::set_activity_settings` 按 `DisplayLevel` 设 thinking 展开：Recent/Full 全文，Summary/Quiet 单行。
- `paints_envelope_header` 纳入 `enabled`。
- `apply_activity_after_turn_end` 在 Recent 档展开最近簇。

### c3125

- `UiModel` 增 `tool_args_full: HashMap<String, String>`；bridge 从 ToolCall 参数填充，不改 `UiEntry::Tool` 的 93 处构造。
- args 模式 hidden/preview/full：preview 5 行 + hint；full 进 viewport；点击 chip 切换。
- 用户消息默认 3 视觉行 + hint，点击展开；`FoldTarget::User`。
- 缓存指纹纳入 args/user 展开态。

### c3130

- 删除 `session_popup_open/id`、pending popup、overlay 渲染、host 方法、chip、动作、keybinding、BDD 场景与 spec r2903 例外。
- `◨` 在输入行右端预留 2 列；命中 2 列；窄档 <80 隐藏。
- spinner `⬢` 固定字形，100ms 在 accent 亮/暗两色间脉冲。

### c3135

- `MessageUpdate` 纳入 stream delta 合批；Tick 33ms 同步。
- 流同步走 `apply_ui_model` 不 `mark_ao_components_stale`；只有 dock/editor 高度变化才标脏。
- Loader 100ms；lab 记录 `lab_ao_stream_delta_perf_report`。

### c3140

- harness：用户冲突提示、默认冲突静默、reload 再提示。
- 产品 snapshot：OSC8 链接含 `\x1b]8;;`；mermaid fence 非裸文本。

### c3145

- fd CJK 文件名测试；产品 harness：fd 不可用回落 read_dir、`@` 空查询列顶层。

## 风险

- 删除详情 popup 会移除已归档行为；同步删除 BDD 场景，避免悬空 binding。
- `tool_args_full` 用 side map 避免 93 处构造改动；缓存键须带 args 展开态。
- spinner 脉冲只改颜色，不改字形，保持 100ms。