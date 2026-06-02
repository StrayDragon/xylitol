
### 技巧

1. **CSI 2026 同步输出**: 所有写操作包裹在 `\x1b[?2026h`/`\x1b[?2026l` 中，防止差分更新过程中的闪烁。`tui.ts#L985,L1230`

2. **16ms 渲染防抖**: `MIN_RENDER_INTERVAL_MS = 16`，配合 `process.nextTick` + `setTimeout` 实现批处理，避免流式 token 每帧都触发渲染。`tui.ts#L253,L524-L541`

3. **CURSOR_MARKER 零宽度标记**: 使用 APC 序列 `\x1b_pi:c\x07` 作为光标位置标记，终端会忽略该序列，TUI 在渲染后提取并定位真实硬件光标。`tui.ts#L84-L90`

4. **Paste Marker 折叠**: 大段粘贴不直接插入文本，而是替换为 `[paste #N +M lines]` 标记，减少渲染行数。标记在 grapheme 分割时被视为原子单元。`editor.ts#L1127-L1139`

5. **tmux 键盘兼容检测**: 启动时检查 `tmux extended-keys` 和 `extended-keys-format` 设置，若不兼容则提示用户修复。`interactive-mode.ts#L809-L854`

6. **Kitty 图片 ID 追踪**: 维护 `previousKittyImageIds` 集合，在 overlay 隐藏或内容变化时正确发送删除命令释放 GPU 内存。`tui.ts#L242,L1008`

7. **行宽溢出保护**: 渲染后检查每行 `visibleWidth`，若超出终端宽度则写入 crash log 并抛出异常，防止终端状态错乱。`tui.ts#L1180-L1207`

8. **Emergency 终端恢复**: 未捕获异常处理器调用 `ui.stop()` 恢复 cooked 模式、光标和所有协议模式，避免终端进入不可用状态。`interactive-mode.ts#L3285-L3302`

9. **输入排空 (drainInput)**: 退出前等待 stdin 清空（最多 1s），防止 Kitty key release 事件泄漏到父 shell。`terminal.ts#L258-L294`

10. **WezTerm 双 ESC 处理**: WezTerm 发送 ESC key 为原始 `\x1b`，release 为完整 Kitty CSI-u。Buffer 检测 `\x1b\x1b` 后面跟 `[`/`O` 等序列时拆分为两个事件。`stdin-buffer.ts#L217-L230`

11. **Termux 高度变化忽略**: Android Termux 的软键盘切换导致频繁高度变化，特殊处理避免每次都全量重绘。`tui.ts#L1038`

12. **Editor 虚拟滚动**: 限制可见区域为终端高度的 30%（最小 5 行），大幅减少长文本的渲染开销。`editor.ts#L428`

13. **stdout 重定向保护**: TUI 运行时劫持 `process.stdout.write` 到 stderr，第三方库的 `console.log` 不会破坏差分渲染状态。`output-guard.ts#L9-L34`

14. **Apple Terminal Shift+Enter 兼容**: Apple Terminal 不发送标准 Shift+Enter 序列，需通过 native modifier 检测 + 序列重写。`terminal.ts#L20-L23`

15. **外部编辑器集成**: `openExternalEditor()` 执行 stop() → spawn editor → start() 循环，确保外部编辑器获得完整终端控制，返回后 `requestRender(true)` 强制全量重绘。`interactive-mode.ts#L3512-L3565`

### 陷阱与解决方案

1. **陷阱: stdin 批量到达导致按键误判**
   - 问题: 终端可能将多个按键序列合并为一次 data 事件，导致 `matchesKey()` 失败。
   - 解决: `StdinBuffer` 将批量输入拆分为完整序列再逐个发出。`stdin-buffer.ts#L192-L255`

2. **陷阱: Kitty 协议退出后序列泄漏**
   - 问题: 退出 raw 模式后，Kitty key release 序列可能被 shell 解释为输入。
   - 解决: `drainInput()` 在停止前等待并丢弃残留输入。`terminal.ts#L258-L294`

3. **陷阱: tmux 转义序列重编码粘贴内容**
   - 问题: tmux 可能将粘贴内容中的控制字符重编码为 Kitty CSI-u 序列。
   - 解决: `handlePaste()` 中正则解码 `\x1b[(\d+);5u` 回原始字符。`editor.ts#L1096-L1101`

4. **陷阱: 渲染行超出终端宽度导致布局错乱**
   - 问题: 组件未正确截断渲染输出，超出终端宽度后终端自动换行导致错位。
   - 解决: `doRender()` 中检查每行宽度，超出则 crash 并写入诊断日志。`tui.ts#L1180-L1207`

5. **陷阱: Windows 下 Shift+Tab 丢失修饰键信息**
   - 问题: libuv 的 `ReadConsoleInputW` 丢弃修饰键状态，Shift+Tab 到达为普通 `\t`。
   - 解决: 动态加载原生模块启用 `ENABLE_VIRTUAL_TERMINAL_INPUT`。`terminal.ts#L228-L256`

6. **陷阱: 全量重绘时回滚缓冲区内容残留**
   - 问题: `\x1b[2J\x1b[H` 清屏但不清理回滚缓冲区，用户上滚会看到旧内容。
   - 解决: 使用 `\x1b[2J\x1b[H\x1b[3J` 同时清理回滚。`tui.ts#L988`

7. **陷阱: Spinner 动画与流式内容竞争渲染**
   - 问题: Spinner 每 80ms 触发 `requestRender()`，同时流式 token 也在触发。
   - 解决: `requestRender()` 的去重机制确保只有一次 pending render。`tui.ts#L519-L520`

---
