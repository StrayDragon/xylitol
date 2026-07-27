# Design: c1620-add-print-eval-foundations

## 权衡

### 1. print trust：CLI 旗标 vs 仅预写 trust store

| 方案 | 利 | 弊 |
|---|---|---|
| A. `print --trust/--no-trust` | 与 tui 对称；容器一行可复现 | 修订 `ce19` |
| B. 只文档化预写 store | 零合约改动 | eval 脆弱、易漏 |

**选定 A。** print 仍 `interactive: false`；有 override 时不走 ChoicePrompt。

### 2. 轮次上限：新硬闸 vs 挂 `should_stop_after_turn`

`ar1`/`ar16`/`rc22` 禁止产品默认 `max_iterations`。社区 eval 需要可选上限。

**选定**：配置键 `session.max_turns: Option<u32>`；组合根在 `Some(n)`（`n ≥ 1`）时安装：

```text
should_stop_after_turn(|ctx| ctx.turn_index + 1 >= n)
```

- `turn_index` 已为 0-based 刚完成 turn（既有 `ShouldStopAfterTurnCtx`）。
- `None` / 缺省：不安装 hook（`ar24` / `no-hook-open-end` 不变）。
- **禁止** schema 再引入 `max_iterations` 名；`rc22` 保留，并注明 `max_turns` 是可选 should_stop 装配，不是迭代硬闸默认。

非法值（0、负数、非整数）：配置加载失败（与其它严格字段一致）。

### 3. Print exit 语义

| 情况 | exit |
|---|---|
| bootstrap / 缺 prompt / 配置失败 | 非 0（已有） |
| run 流中出现 `XyEvent::Error` | **非 0**（本 change） |
| 模型自然结束 / should_stop 结束且无 Error | 0 |
| 工具单次失败但 ReAct 继续 | 0（除非随后 Error） |

不把「未改任何文件」当失败——那是 harness 层（空 patch）的事。

### 4. 装配落点

- Trust：`PrintSurfaceArgs` + `surface_from_command` 传入 `BootstrapInput.trust_override`（对称 tui）。
- `max_turns`：`bootstrap` / composition 在 agent 建成后 `set_should_stop_after_turn`；print 与 tui **共用**（eval 用 print；tui 配了也生效，可接受）。
- Exit：`run_print` / `render_stream` 在见 `Error` 时返回 `Err`，`main` 已有非 0 路径。

## 测试 seam（已确认）

1. **CLI 子进程**：`tests/features/cli-entry.feature`（`ce19` print trust 解析；复用表面旗标词表）。
2. **Print 渲染 / CLI**：`cli-print` feature 或单测——流含 `Error` → 非 0。
3. **AgentRuntime BDD**：`agent-runtime.feature`——配置侧 max_turns 装配后多 turn mock → AgentEnd 且 turn 数封顶；未配置仍开放结束（可复用/扩展 ar24 词表）。

## 非目标

外部 SWE harness、`just eval-swe-smoke`、autosubmit 魔法命令、fake_user。
