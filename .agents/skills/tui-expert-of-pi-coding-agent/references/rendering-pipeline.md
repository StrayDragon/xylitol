# 3. 渲染管线

### 差分渲染策略

TUI 的核心渲染方法是 `doRender()`，它实现了高效的行级差分更新：

1. **全量渲染**: 所有组件调 `render(width)` 生成新行数组 `tui.ts#L970`
2. **Overlay 合成**: 将 overlay 组件渲染到基础内容之上 `tui.ts#L973-L975`
3. **光标位置提取**: 搜索 `CURSOR_MARKER`，计算 IME 光标位置 `tui.ts#L977-L978`
4. **行级 diff**: 逐行比较 `previousLines` 和 `newLines`，找 `firstChanged`/`lastChanged` `tui.ts#L1054-L1067`
5. **增量写入**: 只写入变化的行，用 CSI 序列移动光标 `tui.ts#L1145-L1209`
6. **同步输出**: 整个写操作包裹在 `\x1b[?2026h` / `\x1b[?2026l` 中防止闪烁 `tui.ts#L985-L994`

### 全量重绘触发条件

```mermaid
flowchart TD
    START[doRender] --> FIRST{首次渲染?}
    FIRST -->|是| FULL1["fullRender(false)<br/>不清屏"]
    FIRST -->|否| WCHANGE{宽度变化?}
    WCHANGE -->|是| FULL2["fullRender(true)<br/>清屏+清回滚"]
    WCHANGE -->|否| HCHANGE{高度变化?<br/>(非 Termux)}
    HCHANGE -->|是| FULL3["fullRender(true)"]
    HCHANGE -->|否| SHRINK{内容缩小<br/>&& clearOnShrink?}
    SHRINK -->|是| FULL4["fullRender(true)"]
    SHRINK -->|否| DIFF[行级 Diff]
    DIFF --> ANY{有变化?}
    ANY -->|否| CURSOR[仅更新硬件光标]
    ANY -->|是| ABOVE{变化行<br/>在视口上方?}
    ABOVE -->|是| FULL5["fullRender(true)"]
    ABOVE -->|否| INCR["增量写入<br/>firstChanged..lastChanged"]
```

### 流式内容局部刷新

流式 token 追加时，由于只在最后几行发生变化，diff 算法自然地只更新尾部行。`AssistantMessageComponent` 在每次 `updateContent()` 后调用 `requestRender()`，TUI 的 16ms 防抖确保高频 token 不会每次都渲染。`interactive-mode.ts#L2724-L2756`

### 终端能力检测

- **Kitty 协议**: 启动时探测 `terminal.ts#L193-L203`
- **图片支持**: 通过 `getCapabilities()` 检测 Kitty/iTerm2 图片协议 `tui.ts#L464-L471`
- **Cell 尺寸查询**: `\x1b[16t` 查询像素级 cell 大小（仅图片渲染需要） `tui.ts#L463-L471`
- **Windows VT 输入**: 动态加载 `.node` 原生模块启用 `ENABLE_VIRTUAL_TERMINAL_INPUT` `terminal.ts#L211-L239`

### Resize 自适应

- SIGWINCH 信号触发 `requestRender()`，但 Unix 下 suspend/resume 会丢失信号
- 启动时主动发送 `SIGWINCH` 刷新尺寸 `terminal.ts#L123-L125`
- 宽度变化触发全量重绘（因为自动换行改变） `tui.ts#L1029-L1032`
- 高度变化通常触发全量重绘，但 Termux 环境例外（软键盘导致频繁高度变化） `tui.ts#L1038-L1042`

---
