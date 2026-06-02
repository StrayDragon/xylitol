
### 后台任务与 UI 通信

采用 Observer 模式：`AgentSession` 发出事件，`InteractiveMode` 订阅并更新 UI 组件。

### 加载指示器 (Loader)

```typescript
// loader.ts#L17-L92
class Loader extends Text {
    start(): void {           // 开始动画
        this.updateDisplay();
        this.restartAnimation();
    }
    private restartAnimation(): void {
        this.intervalId = setInterval(() => {
            this.currentFrame = (this.currentFrame + 1) % this.frames.length;
            this.updateDisplay(); // 更新文本 → requestRender()
        }, this.intervalMs);     // 默认 80ms
    }
}
```

默认使用 Braille 点阵动画 `["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]`，每 80ms 切换一帧。 `loader.ts#L12-L13`

### Token 打字机效果

流式 token 通过高频 `message_update` 事件实现。每次事件更新 `AssistantMessageComponent` 的 Markdown 渲染，TUI 的 16ms 防抖自动合并高频更新。用户体验为"逐字符显示"，但实际是"每 ~16ms 一次批量渲染"。

### 任务取消

- **Ctrl+C**: 双击 500ms 内执行 `shutdown()`，单击清空编辑器 `interactive-mode.ts#L3229-L3237`
- **Escape**: 流式中断开并恢复排队消息 `interactive-mode.ts#L2380-L2382`
- **Ctrl+Z**: SIGTSTP 信号处理，suspend 进程 `interactive-mode.ts#L2410`

### Ctrl+Z Suspend/Resume 模式

`handleCtrlZ()` 实现了完整的进程挂起与恢复流程，包含多个细节防护：

```typescript
// interactive-mode.ts#L3358-L3393
handleCtrlZ(): void {
    // 1. keepalive 定时器防止 Node 事件循环退出
    //    stop() 后可能没有 ref'ed handles，进程会在 fg 前退出
    const suspendKeepAlive = setInterval(() => {}, 2 ** 30);

    // 2. 挂起期间忽略 SIGINT，防止 Ctrl+C 杀死后台进程
    const ignoreSigint = () => {};
    process.on("SIGINT", ignoreSigint);

    // 3. 注册 SIGCONT 恢复处理器
    process.once("SIGCONT", () => {
        clearInterval(suspendKeepAlive);
        process.removeListener("SIGINT", ignoreSigint);
        this.ui.start();            // 重新进入 raw 模式
        this.ui.requestRender(true); // 强制全量重绘
    });

    this.ui.stop();                  // 恢复 cooked 模式
    process.kill(0, "SIGTSTP");      // 发送 SIGTSTP 到进程组
}
```

### stdout 重定向 (Output Guard)

TUI 运行期间，第三方库的 `console.log()` 会向 stdout 写入文本，破坏差分渲染的状态追踪。`output-guard.ts` 通过替换 `process.stdout.write` 将所有 stdout 输出重定向到 stderr：

```typescript
// output-guard.ts#L9-L34
function takeOverStdout(): void {
    const rawStdoutWrite = process.stdout.write.bind(process.stdout);
    const rawStderrWrite = process.stderr.write.bind(process.stderr);

    // 将所有 stdout.write 调用重定向到 stderr
    process.stdout.write = (chunk, ...) => rawStderrWrite(String(chunk), ...);

    // 保存原始 write 供 TUI 自身使用
    // writeRawStdout() 绕过重定向直接写 stdout
}
```

TUI 使用 `writeRawStdout()` 直接写入 stdout 进行渲染，确保只有受控的 ANSI 序列到达终端。`interactive-mode.ts` 在 `ui.start()` 前调用 `takeOverStdout()`，`shutdown()` 时调用 `restoreStdout()` 恢复。

### 进度指示

`Terminal.setProgress(active)` 使用 OSC 9;4 序列在终端图标/任务栏显示进度动画，带 1s 保活心跳。 `terminal.ts#L398-L412`

### 跨平台剪贴板

`clipboard.ts` 实现了多层降级的剪贴板写入：

1. **Native addon** (`clipboard-rs`): macOS/Windows 直接调用系统 API（Linux 跳过，因 X11-only 且不保持选区所有权）
2. **平台工具**: `pbcopy`(macOS) / `clip`(Windows) / `termux-clipboard-set` / `wl-copy`(Wayland, spawn 异步) / `xclip`/`xsel`(X11)
3. **OSC 52 终端序列**: SSH 远程会话时写入 `\x1b]52;c;<base64>\x07`，利用终端转发到本地剪贴板
4. 远程会话 (`SSH_CONNECTION`) 时同时使用 native + OSC 52，确保双端可用

`clipboard-image.ts` 提供剪贴板图片读取，同样支持 Wayland (`wl-paste --type image/png`) 和 X11 (`xclip -selection clipboard -target image/png`)。

---
