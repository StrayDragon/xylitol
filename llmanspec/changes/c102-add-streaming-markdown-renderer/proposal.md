---
depends_on: []
---

# c102-add-streaming-markdown-renderer

## Why

当前 `ChatComponent` 在每个 `TextDelta` 事件到来时标记 `dirty=true`，导致下一帧需要对整个消息内容执行完整 `markdown.render()` 调用。对于长对话（数千字），这意味着：

- LLM 流式输出期间，每帧都执行全量 Markdown 解析 + 语法高亮
- CPU 开销随对话长度线性增长，造成 TUI 卡顿和帧率下降
- 渲染结果每帧可能抖动（未完成的 Markdown 结构导致解析差异）

leaf-core 新增的 `StreamingRenderer` 提供了三层渐进式渲染策略：

1. **防抖全量重解析**（debounced reparse）— 限制渲染频率
2. **增量 diff 检测**（stable prefix detection）— 仅暴露变更区域
3. **双阶段渲染**（dual-phase）— 已完成段落精确渲染 + 进行中尾部轻量样式

## What Changes

1. **ChatComponent 引入 `StreamingRenderer`**
   - 新增 `Option<StreamingRenderer>` 字段，流式消息期间激活
   - `append_assistant_delta` → 调用 `streaming.push(delta)` 而非直接拼接 content
   - render 循环中调用 `streaming.tick()` 获取增量更新

2. **渲染路径优化**
   - 利用 `StreamUpdate.changed_from` 仅重绘变更行
   - 启用 dual-phase 模式减少进行中段落的解析开销
   - `finish_streaming` 时调用 `streaming.finish()` 获取最终精确输出

3. **回退机制**
   - 当 `raw_output=true` 时跳过 StreamingRenderer，保持原有直接 content 追加逻辑
   - `set_width` 变化时通知 StreamingRenderer 刷新宽度

## Capabilities

- `markdown-rendering`: 新增流式渲染需求

## Impact

- **性能提升**：流式输出期间 CPU 使用降低 60-80%（基于 leaf 基准测试）
- **视觉稳定**：防抖 + 稳定前缀检测减少渲染抖动
- **接口兼容**：`MarkdownRenderer` pub(crate) API 不变，仅 ChatComponent 内部逻辑调整
- **新依赖**：无（StreamingRenderer 已在 leaf-core 中）
