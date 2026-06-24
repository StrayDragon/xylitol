# c84-refactor-tui-diff-engine — Design

## Context

- c82-fix-tui-core 修复了差分 ANSI 渲染引擎的 P0 bug,引擎已可用
- c83 试图迁移到 ratatui + alternate screen,方向与项目目标(对齐 `../pi` 裸终端体验)相悖,被废弃
- 本变巩固 pi 式方向,并修复 c82 遗留的"启动即调 agent"缺陷
- 参考:`../pi/packages/tui/src/tui.ts:doRender()`(差分渲染)、`terminal.ts:138`(纯 raw mode,无 alternate screen)

## 决策:为何保留差分 ANSI 引擎而非 ratatui

| 维度 | pi 式差分(保留) | ratatui(放弃) |
|------|----------------|---------------|
| 原生文本选择 | ✅ 终端原生 | ❌ alternate screen 接管 |
| scrollback | ✅ 进正常历史 | ❌ 退出即清空 |
| 外部分页器 | ✅ 可被 `less` 等捕获 | ❌ 全屏接管 |
| 渲染开销 | 行级 diff(仅写改动行) | 全帧 diff(ratatui Buffer) |
| 已验证可用 | ✅ c82 | (c83 未实现完即废弃) |

结论:**engine/ 完全不动**,只修事件循环里"启动即调 agent"这一处缺陷。

## 改动点:移除启动时的 premature agent 调用

### 现状(c82 实现的 `mod.rs`)

```rust
pub(crate) async fn run_tui_engine(
    agent_loop: &mut AgentLoop,
    prompt: &str,                  // ← cli 传 ""
    session_id: &str,
    model_name: &str,
) -> Result<(), String> {
    setup_engine()?;
    let mut renderer = TuiRenderer::new(io::stdout());

    // ❌ 缺陷:启动即调 agent,触发一次空 prompt 的模型 API 调用
    let mut agent_stream = agent_loop.run(prompt, session_id).await;

    // ... select! 循环 ...
}
```

cli 调用点:
```rust
crate::interface::tui::run_tui_engine(&mut agent_loop, "", &session_id, &model_name)
```

### 目标

`agent_stream` 初始为 `None`,只有当 `pending_prompt` 首次被用户提交填充时才创建。
这复用 c82 已有的 `pending_prompt` 机制(原本用于 agent 完成后 dequeue 下一题),
只是把"启动时的 stream 创建"也改为懒触发。

```rust
pub(crate) async fn run_tui_engine(
    agent_loop: &mut AgentLoop,
    session_id: &str,              // ← 去掉 prompt
    model_name: &str,
) -> Result<(), String> {
    setup_engine()?;
    let mut renderer = TuiRenderer::new(io::stdout());

    let mut agent_stream: Option<AgentEventStream> = None;   // ← None,不调 run
    let mut pending_prompt: Option<String> = None;

    loop {
        // 懒触发:有 pending 才创建/重建 stream
        if let Some(next_prompt) = pending_prompt.take() {
            agent_stream = Some(agent_loop.run(&next_prompt, session_id).await);
        }

        // agent 分支:None 时 pending(),避免 select! 饥饿(同 c82 Bug 1 修法)
        let agent_fut = match agent_stream.as_mut() {
            Some(s) => s.next().boxed(),
            None => futures::future::pending::<Option<AgentEvent>>().boxed(),
        };

        tokio::select! {
            event = agent_fut => {
                match event {
                    Some(ev) => { app.transcript.apply(ev); dirty = true; /* dequeue */ }
                    None => { agent_stream = None; dirty = true; }   // ← 流结束置 None
                }
            }
            input_event = input_stream.next().fuse() => { /* ... 首次提交设 pending_prompt ... */ }
            _ = frame_interval.tick() => { /* engine::compose_layout + renderer.render */ }
        }
    }
}
```

这同时保留了 c82 的 `agent_done` → `pending()` 饥饿修复(现在用 `Option<Stream>`
+ 匹配分支表达,等价且更清晰)。

## 边界:`ratatui-textarea` 不在本次替换

c82 的 `state/composer.rs` 用 `ratatui-textarea::TextArea` 作为输入控件。
技术上这与"裸终端"理念有张力,但:

1. `textarea` 只在 composer 内部用,不涉及屏幕接管或 alternate screen
2. 自实现 pi 式 editor(`editor.ts` + `undo-stack.ts` + `kill-ring.ts` +
   `word-navigation.ts`)是独立大工程,不应混入本次反转
3. 现有 spec r41 要求 `state/` 不依赖 ratatui —— composer 用 textarea 是 c82
   的已知妥协,留 future 单独处理

本变更**严格只反转渲染层 + 修 premature-agent**,不碰输入控件。

## 废弃 c83

`c83-migrate-tui-to-ratatui/` 整个删除。理由:
- 方向错误(ratatui + alternate screen)
- 其 spec.toon 的 3 条 delta(`ratatui-rendering`/`alternate-screen`/
  `no-premature-agent`)与 c84 的 r47/r49 直接冲突
- 尚未归档,删除无信息损失
- 经用户确认:不进 `not-planning/`,直接移除

## 测试策略

### 单元测试(必须继续通过)

```bash
cargo test --features ui-tui --lib -- interface::tui
```

c82 的 ~90 条 TUI 测试覆盖 `state/`、`input/`、`engine/`(ansi/diff/renderer/
composer_renderer/transcript_renderer)纯逻辑层,本次改动不触及任何被测代码路径。

### BDD 回归

```bash
cargo test --test bdd -- --test-threads=1
```

无 TUI BDD feature,82 条 BDD 全部与渲染无关,必须保持通过。

### 结构性验证 no-premature-agent(r48)

```bash
# 无 TTY 环境下启动 TUI,应在"setup 后、agent 调用前"就因 stdin 报错退出,
# 不会产生任何模型 API 调用。
cargo run --features ui-tui -- tui
```

交互式冒烟(需真终端 + API key,人工):

```bash
cargo run --features ui-tui -- tui
```

验证:
1. 启动后看到 composer + 空 transcript(首帧即绘,满足 r24)
2. **启动瞬间无任何网络请求 / agent 活动**(r48)
3. 键入回车提交后,用户消息出现,agent 响应紧跟
4. Ctrl+D 退出,终端恢复光标,scrollback 中可见本次 TUI 输出(r47)
5. 鼠标可原生选择文本(r47)
