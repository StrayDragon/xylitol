# Design: c1200 MCP 启动体验（薄）

## 目标体验

| 路径 | 用户应感到 |
|---|---|
| **新会话** | 几乎立刻进 TUI；头卡 mcp 行从 `connecting…` → `N connected`；可先聊（builtins） |
| **CLI `--session` resume** | 同样立刻进 TUI；**历史先投影**（c1790 Tool 单块等）；MCP 在旁连接，不挡看旧会话 |
| **面内 `/session-resume`** | 切会话只换 transcript；**不**重演启动连 MCP |
| **print** | 首 prompt 不被 MCP 长时间挡住；诊断在 stderr/log |

## 架构（意向）

```text
今日:
  bootstrap_mcp().await ──► run(TUI) ──► [resume? rebuild]

本波:
  enable_reload_state
       │
       ├─► run(TUI) 立刻 ──► resume? rebuild transcript
       │         ▲
       │         │ 进度 / ready 刷新 loaded-resources
       └─► spawn MCP connect (并行 servers) ──► set_tools 热合并
```

## Chrome 落点（词汇表对齐）

| 信号 | 落点 | 理由 |
|---|---|---|
| 连接进度 | **loaded-resources · mcp 行**（atc18 / ath23） | 属启动资源态；不刷 scrollback；不占 busy status / 下轮预告 |
| ready | 同槽刷成 connected 摘要 | ath23「启动后刷新」扩展为进行中也可刷 |
| 单服失败 | mcp 行附短诊断 / diagnostics | mcp4 |
| 全失败且已配置 | mcp 行失败态；MAY **一条**滚动提示 | 避免进度刷墙 |

**禁止**：把 `MCP connecting 1/3` 每步尾随成多条 ScrollNotice；用下轮预告冒充 MCP 进度。

### mcp 行文案意向（实现可微调用语）

```text
connecting 1/3 · foo
2 connected · foo(3) · bar(1)
3 configured · 0 connected · foo: timed out   # 失败可感
```

`LoadedResourcesSnapshot` 宜增加 phase（或等价字段），避免 host 猜字符串。

## 首轮工具策略（B）

- 连接中：ToolSet = builtins（+ 已连上的可增量合并，若实现简单）
- **MUST**：ready 后 `set_tools` 合并 mcp:，**下一** `run` 可见
- **MUST NOT** 为等 MCP 阻塞 TUI 首帧或 CLI resume 的 rebuild

可选增强（非本波 MUST）：已连上的 server 工具可中途增量注册——若做，须测竞态。

## 并行连接

- `connect_servers`：`join_all`（或等价）替代串行 `for` await
- 超时：升格时定默认（或先依赖 transport/OS）；超时记 diagnostics，不拖死其它 server

## `/reload`（本波边界）

- ath20 步骤顺序/部分成功 **不变**
- 实现上可复用并行 connect + 快照刷新
- **不做**：进行中禁输入、禁二次 reload、可取消、专用 loader 动画（→ 留 c1205）

## 与 resume 的交互细节

1. **CLI restore**：`tui::run` 内 `apply_cli_restored_session` 与 MCP task **并发**；rebuild 完成即可滚历史；MCP 稍后更新头卡。
2. **面内 resume**：`switch_session` 路径不调用 bootstrap；头卡保持当前 MCP 态。
3. **大历史**：rebuild 成本与 MCP 无关；本波不解决 activity-fold（c1760）。

## 非目标

- c1205；假 `Worked for`；改 wire；Trust/allowlist UI
