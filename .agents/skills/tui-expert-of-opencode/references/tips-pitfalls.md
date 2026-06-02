# 关键技巧与避坑指南

## 技巧（12 条）

1. **二分查找有序数组**：所有服务端同步的有序数据使用 `Binary.search` + `produce` 确保插入位置正确且 O(log n)。`sync.tsx#L142-L149`

2. **事件批处理减少渲染**：SDK 层将 16ms 内的 SSE 事件缓冲后用 `batch()` 一次性发射，避免每条事件触发独立渲染。`sdk.tsx#L46-L57`

3. **Bootstrap 分阶段加载**：先加载关键数据（providers/agents/config）至 `partial`，再异步加载次要数据（sessions/commands/lsp），UI 可在 partial 时渲染。`sync.tsx#L378-L479`

4. **消息自动清理**：单个 session 超过 100 条消息时清理最旧的 + 对应 parts，防止内存泄漏。`sync.tsx#L271-L289`

5. **createSimpleContext 工厂**：统一的 context 创建模式，自动处理 ready 状态延迟渲染。`context/helper.tsx#L6-L25`

6. **Mode Stack 模式管理**：用栈管理 UI 模式（base/modal），每个 push 返回 dispose 函数，防止模式泄漏。`keymap.tsx#L41-L88`

7. **Dialog 焦点保存恢复**：打开 dialog 时保存 `currentFocusedRenderable`，关闭时用 DFS 查找并恢复。`ui/dialog.tsx#L84-L99`

8. **paste 长文本折叠**：≥3 行或 >150 字符的粘贴自动折叠为 `[Pasted ~N lines]`，通过 extmark 保留原始数据。`prompt/index.tsx#L1305-L1311`

9. **submit 防重入**：用 `submitting` 标志防止 IME 双触发或快速连按导致空消息发送。`prompt/index.tsx#L993-L1008`

10. **主题灰度自适应**：根据终端背景亮度自动生成 12 级灰度和柔和文本色，确保在任何终端配色下可读。`theme.tsx#L633-L716`

11. **Win32 Ctrl+C 守卫**：Hook `setRawMode` + 100ms 轮询，确保 Windows 上 ENABLE_PROCESSED_INPUT 不被运行时重置。`win32.ts#L69-L130`

12. **终端标题同步**：`createEffect` 监听路由变化动态设置终端窗口标题，40 字符截断。`app.tsx#L352-L375`

## 陷阱与解决方案（7 个）

1. **陷阱：IME 输入与 submit 时序竞争**
   - **现象**：韩文等 IME 输入的最后一个字符可能尚未 flush 到 `plainText` 就触发了 submit
   - **解决**：使用 `setTimeout(() => setTimeout(() => submit(), 0), 0)` 双延迟等待 IME flush。`prompt/index.tsx#L1510-L1511`

2. **陷阱：SolidJS Store 直接赋值不触发响应式更新**
   - **现象**：直接修改 `part[field]` 在某些情况下不会触发细粒度更新
   - **解决**：使用 `produce()` 包裹修改逻辑，或使用路径式 `setStore("part", messageID, index, reconcile(...))`。`sync.tsx#L264-L270`

3. **陷阱：Dialog 打开时 Prompt 抢焦点**
   - **现象**：路由切换或插件更新可能导致隐藏的 Prompt 重新挂载并抢占焦点
   - **解决**：在 focus effect 中检查 `dialog.stack.length`，有 dialog 时不聚焦。`prompt/index.tsx#L698-L708`

4. **陷阱：Windows ConPTY 粘贴发送空内容**
   - **现象**：Windows Terminal <1.25 在图像剪贴板时发送空 bracketed paste
   - **解决**：检测空粘贴内容后降级为 `prompt.paste` 命令走剪贴板 API。`prompt/index.tsx#L1526-L1529`

5. **陷阱：session 创建与 navigate 的竞态**
   - **现象**：新建 session 后立即 navigate 可能在 sync 完成前触发
   - **解决**：用 `setTimeout(50ms)` 延迟 navigate 确保消息已发送。`prompt/index.tsx#L1215-L1223`

6. **陷阱：Terminal title 设置在 SIGTSTP 恢复后丢失**
   - **现象**：suspend/resume 后终端标题被重置
   - **解决**：监听 `SIGCONT` 后调用 `renderer.resume()` 恢复状态。`app.tsx#L744-L749`

7. **陷阱：主题解析循环引用**
   - **现象**：Theme JSON 的 `defs` 字段互相引用导致无限递归
   - **解决**：在 `resolveColor` 中维护 `chain` 数组检测循环引用并抛出错误。`theme.tsx#L208-L209`
