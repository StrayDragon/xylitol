# Design: c1200 MCP 启动体验（薄）

## 目标体验

| 路径 | 用户应感到 |
|---|---|
| **新会话** | 几乎立刻进 TUI；头卡 mcp `connecting…`；可看界面 / slash / 滚空白；**agent prompt 暂不可提交**，ready 后可聊 |
| **CLI `--session` resume** | 立刻进 TUI；**历史先投影**；可滚读旧会话、slash、面内再 resume；**agent prompt 仍闸到 MCP 结算** |
| **面内 `/session-resume`** | 切会话只换 transcript；**不**重演启动连 MCP；若启动波仍 connecting，闸规则同上 |
| **print** | 无浏览态：首 prompt **可**等 MCP 结算后再跑（或短超时后带已连上的）；进度 stderr/log |

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

## 输入闸策略（A · 已拍）

连接中（`mcp_servers` 非空且尚未结算）：

| 允许 | 闸住 |
|---|---|
| 看/滚 transcript（含 CLI resume 投影） | **agent 普通 prompt 提交**（`run`） |
| 打字进 editor（不提交） | **bang `!`** |
| slash 见下表白名单 | slash 见下表黑名单 |

拒绝时：短滚动提示（`MCP still connecting — …`），**MUST NOT** 静默吞 Enter。

结算后：`set_tools` 热合并 → **解闸**。

### Slash 白名单（已锁 · 2026-07-31）

**独立于** `busy_slash_policy`（agent busy 拒 `/session-resume`，此处要放行）。

| Permit | 命令 |
|---|---|
| **Allow** | `/exit`、`/session`、`/history-copy-last`、`/session-export`、`/session-resume`、`/session-new`、`/session-clone`、`/session-name`、`/session-import`、`/session-tree`、`/session-fork`、`/theme`、`/model`（开槽+有参）、`/trust`、`/debug`、Usage 提示 |
| **Reject** | `/reload`、`/session-compact` |

### Rust 扩展方式（可维护）

**单次穷尽 `match` 填双列**，新增 `PendingSlash` 变体时编译器迫使两列都填：

```rust
pub struct SlashAllowances {
    pub when_agent_busy: SlashPermit,
    pub when_mcp_connecting: SlashPermit,
}

pub fn slash_allowances(slash: &PendingSlash) -> SlashAllowances { match slash { /* 穷尽 */ } }
```

- `busy_slash_policy` / `mcp_connecting_slash_policy` 只是列投影，禁止再维护第二份平行 match。
- 将来第三闸（如 c1205 reload-in-flight）→ **加一列字段**，仍一处穷尽。

**为何不用 B（先 builtins 聊）**：见下节；本波先 A，后续可再议软开。

## 若将来回到 B：系统提示 / 工具插入会不会「遗忘·幻觉」？

本仓库每轮 `run` 会把 **当时** `ToolSet` 编进 provider 请求（tools 参数 + 当前 system），**不是**「只在第一轮写死、以后靠模型记忆工具表」。

| 做法 | 对 provider | 风险 |
|---|---|---|
| **只改下一轮 tools 列表**（热合并，不改历史消息） | 正常；新一轮 schema 以请求为准 | 低。模型偶发仍提未声明工具名 → 运行时无此 tool |
| **往历史里插入一条「MCP ready」system/user 伪消息** | 污染 transcript；多轮后更易偏 | **较高**；且与「面不伪造对话」不符 |
| **改 system 正文中途追加「现已有 mcp:…」长列表** | 每轮 system 可变，一般可行 | 中：与 tools 参数重复；列表很长时吵 |

结论：B 若只做 **下一请求带上新 tools**（不插假系统对话），不算「遗忘」——历史还在，只是工具目录变了。真正麻烦是 **首轮用户以为 MCP 已可用 / 模型按用户口头点名幻觉调用**。A 用闸避开首轮不一致；以后若开 B，优先「tools 热合并 + 头卡可感」，**避免**往 session 里塞假 system 行。

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
