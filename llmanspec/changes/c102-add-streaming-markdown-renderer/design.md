# c102-add-streaming-markdown-renderer Design

## Decision 1: StreamingRenderer 生命周期

**选项：**
- A) 全局单例 StreamingRenderer（随 ChatComponent 创建）
- B) 每条流式消息创建一个 StreamingRenderer（用完 reset）

**决定：** B — 每条消息使用一个 StreamingRenderer 实例（通过 `reset()` 复用）

**理由：**
- StreamingRenderer 的 `buffer` 和 `committed_lines` 与单条消息绑定
- `reset()` 方法允许在同一实例上重新开始，避免重复分配语法/主题资产
- 实际实现为 ChatComponent 持有一个 StreamingRenderer 字段，流式开始时 reset

## Decision 2: 渲染模式选择

**选项：**
- A) 仅使用 debounced full reparse
- B) 启用 dual-phase（已完成段落精确 + 尾部近似）
- C) 可配置，默认 dual-phase

**决定：** C — 默认启用 dual-phase，可通过配置关闭

**理由：**
- dual-phase 对长对话性能提升最大（已完成段落缓存）
- 近似尾部渲染视觉上可接受（仅影响正在输入的最后几行）
- 用户如有洁癖可关闭回退到 full reparse

## Decision 3: 防抖间隔

**决定：** 使用 leaf-core 默认的 150ms，不额外配置

**理由：**
- 150ms 对应约 6-7 FPS 的渲染更新，视觉上流畅
- TUI 帧率通常为 30-60 FPS，150ms 防抖避免过度渲染
- 如未来需要可通过 `with_debounce()` 调整

## Decision 4: changed_from 优化

**决定：** 初期不实现行级增量重绘，仅利用 `changed_from` 优化缓存失效

**理由：**
- ratatui 的 buffer diff 机制本身已做终端级增量刷新
- `changed_from` 主要用于 ChatComponent 内部的 `rendered_lines` 缓存策略
- 避免过度优化引入复杂度；后续可在 TUI 渲染层利用此信息
