# Handoff: c84-refactor-tui-diff-engine 实现

## 现状

**方向已反转**:c83-migrate-tui-to-ratatui 被废弃删除(经用户确认,不进 not-planning)。
当前活跃变更:**c84-refactor-tui-diff-engine** —— 保留并巩固 pi 式差分 ANSI 渲染引擎,
拒绝 ratatui 现成组件 + alternate screen 方向,并修复 c82 遗留的"启动即调 agent"缺陷。

**变更路径**:`llmanspec/changes/c84-refactor-tui-diff-engine/`
**实现前请阅读**:`proposal.md`、`design.md`、`tasks.md`

> 注:c84 的代码任务**已全部完成**(见下方"已完成"),strict 校验已过。
> 本文档保留供后续验证 / 归档 / 接力时参考。

## 架构决策(为何选 pi 式而非 ratatui)

| 维度 | pi 式差分(保留) | ratatui(放弃) |
|------|----------------|---------------|
| 原生文本选择 | ✅ 终端原生 | ❌ alternate screen 接管 |
| scrollback | ✅ 进正常历史 | ❌ 退出即清空 |
| 渲染开销 | 行级 diff(仅写改动行) | 全帧 diff(ratatui Buffer) |
| 参考 | `../pi/packages/tui/src/tui.ts:doRender()` | — |

**结论**:`engine/`(diff.rs/renderer.rs/ansi.rs/composer_renderer.rs/
transcript_renderer.rs)**完全不动**。只修事件循环里"启动即调 agent"这一处。

## 已完成的代码改动

### 1. `src/interface/tui/mod.rs` —— 移除 premature agent 启动

**旧行为**(c82 实现):
```rust
pub(crate) async fn run_tui_engine(
    agent_loop: &mut AgentLoop,
    prompt: &str,                  // ← cli 传 ""
    ...
) -> ... {
    // ❌ 启动即调 agent,触发一次空 prompt 的模型 API 调用
    let mut agent_stream = agent_loop.run(prompt, session_id).await;
    let mut agent_done = false;    // c82 的饥饿修复标志
    ...
}
```

**新行为**(c84):
```rust
pub(crate) async fn run_tui_engine(
    agent_loop: &mut AgentLoop,
    session_id: &str,              // ← 去掉 prompt
    ...
) -> ... {
    let mut agent_stream: Option<AgentEventStream> = None;  // ← None,不调 run
    let mut pending_prompt: Option<String> = None;
    loop {
        // 懒触发:有 pending 才创建/重建 stream
        if let Some(next_prompt) = pending_prompt.take() {
            agent_stream = Some(agent_loop.run(&next_prompt, session_id).await);
        }
        // None 时 pending(),复用 c82 的饥饿修复(用 Option 表达,等价于旧 agent_done)
        let agent_fut = match agent_stream.as_mut() {
            Some(s) => s.next().boxed(),
            None => futures::future::pending::<Option<AgentEvent>>().boxed(),
        };
        ...
        // 流结束时: agent_stream = None; (而非 agent_done = true)
    }
}
```

### 2. `src/interface/cli/mod.rs` —— 调用点去 prompt

```rust
// 旧: run_tui_engine(&mut agent_loop, "", &session_id, &model_name)
// 新:
crate::interface::tui::run_tui_engine(&mut agent_loop, &session_id, &model_name)
```

### 验证结果

| 检查 | 结果 |
|------|------|
| `cargo build --features ui-tui` | ✅ |
| TUI 单测(c82 的 112 条) | ✅ 全过,零修改 |
| BDD 回归(82 条) | ✅ 全过 |
| `just fmt && just lint && just test`(632 测试) | ✅ |
| r48 结构性验证(strace:无 TTY 启动无 `connect`) | ✅ 无 API 调用 |
| `llman sdd validate c84 ... --strict` | ✅ |

## 已知遗留(future.md 候选,不在 c84 做)

1. **composer 的 `ratatui-textarea`**:c82 用它做输入控件,与"裸终端"理念有张力。
   自实现 pi 式 editor(`editor.ts`+`undo-stack.ts`+`kill-ring.ts`+`word-navigation.ts`)
   是独立大工程,留 future。注意 spec r41 要求 `state/` 不依赖 ratatui —— composer 用
   textarea 是已知妥协。
2. **清理 `Cargo.toml` 的 ratatui 依赖**:差分渲染路径已不需要 ratatui,但 textarea
   还在用,等(1)完成后可彻底移除。
3. **改进 `compose_layout()` 行裁剪**:transcript 行超出视口时仍是简单 truncate,
   可做更精细的 viewport 偏移(非阻塞优化)。

## 配置 bug(独立问题,与 c84 无关)

用户报告"似乎没用 .xylitol 配置,默认还是 gpt5.4"。已定位:
- **主因**:`<project>/.xylitol/config.local.yaml` 顶层 key 写成 `model:`(单数),
  但代码是 `#[serde(rename = "models")]`(复数),导致 `qwen` 条目被静默丢弃,
  回退到环境变量兜底 `default_model_id_for_provider("openai") = "gpt-5.4"`。
- **次因**:`registry.rs:90` 的兜底默认值本身是错误占位符(`gpt-5.4`/
  `claude-opus-4-8` 都不存在)。
- **隐患**:本地无鉴权 provider(`http://tufa:50256/v1`)不应强制要 OPENAI_API_KEY。

建议另开变更(如 `c85-fix-config-model-loading`)处理,**不在 c84 混做**。

## 代码入口

| 文件 | 用途 |
|------|------|
| `src/interface/tui/mod.rs` | 事件循环(已改) |
| `src/interface/cli/mod.rs` | CLI 入口(已改调用点) |
| `src/interface/tui/engine/*` | 差分渲染(不动) |
| `src/interface/tui/state/*` | 纯状态层(不动) |
| `src/interface/tui/input/*` | 纯输入层(不动) |

## 验证

```bash
cargo build --features ui-tui
cargo test --features ui-tui --lib -- interface::tui    # 112 条
cargo test --test bdd -- --test-threads=1               # 82 条
# 无 TTY 下启动应立即报错退出,无 API 调用:
./target/debug/xylitol tui </dev/null
# 交互式(需真终端 + API key):
cargo run --features ui-tui -- tui
```

成功标准:
1. 启动后看到 composer + 空 transcript(首帧即绘)
2. **启动瞬间无任何网络请求 / agent 活动**(r48)
3. 键入回车提交后,用户消息出现,agent 响应紧跟
4. Ctrl+D 退出,终端恢复光标,scrollback 中可见本次 TUI 输出
5. 鼠标可原生选择文本(r47)
