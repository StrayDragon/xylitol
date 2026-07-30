# Design: c1210 MCP hot-merge ungate

## 目标体验

| 时刻 | 用户 |
|---|---|
| TUI 已开、MCP connecting | 可提交 prompt / bang / 多数 slash（含 `/reload`）；头卡 `connecting i/n` |
| MCP settle | 无假对话行；ToolSet overlay；**下一轮** provider `tools` 含 MCP |
| `/reload` 再连 MCP | 旧 client/子进程先 shutdown；新 manager 替换；无静默堆叠残留进程 |

## 推翻闸 A

删除（或恒为 false）host 侧 `mcp_blocks_agent` 对：

- 普通 prompt Enter
- bang `!` / `!!`
- `mcp_connecting_slash_policy` Reject 路径

**保留**：agent **busy** 既有闸（含 bang 拒、slash busy 列）——与 MCP connecting 无关。

**Slash 表**：去掉 `when_mcp_connecting` 字段与投影函数；`slash_allowances` 只保留 `when_agent_busy`（若仅剩一列，可缩回单一 `SlashPermit` 穷尽表——以「仍一处穷尽、无平行 match」为准）。

## Tools：单一写入口（防重复）

今日 `ToolSet::merge` = 裸 `extend`，二次 settle 会叠名。

```text
rebuild_agent_tools(builtins, mcp_discovered) -> ToolSet
  = ToolSet::from_iter(builtins).overlay_by_name(mcp_discovered)
```

| 规则 | |
|---|---|
| MUST | settle / MCP 段 `/reload` 成功后 **只** 经该入口 `set_tools` |
| MUST | `overlay_by_name`：同名后写覆盖前写；结果 tool **名唯一** |
| MUST NOT | 对可能已含 `mcp:` 的 live set 再 `merge`/`extend` |
| SHOULD | Driver 私有或 `agent/tools` 上的 `overlay_by_name` + 组合函数；单测钉二次 rebuild |

每轮 `run` 仍把**当时** ToolSet 编进请求（既有）；符合 ar6 next-turn。

## 进程生命周期（放开 reload 的配套）

```text
replace_mcp:
  old = take manager
  connect_and_discover → new manager + tools
  rebuild_agent_tools(builtins, tools) → set_tools
  old.shutdown().await   // 今日 settle 已有类似顺序；reload 路径对齐并测
```

- shutdown 失败：log + diagnostics / 短提示可感；**尽量**不留下「新旧双活」stdio。
- 本波 **最小**关停/替换；二次 reload 进行中的输入锁 / spinner → 仍 **c1205**。

## SYSTEM（本波薄收紧；大重构 c1220）

默认 `build_system_prompt` 路径：

- `Available tools:` **仅** builtins（非 `mcp:` 前缀）
- 固定一句（意向心智）：MCP/custom 以**本轮请求 tools 列表**为准，按精确名调用
- MUST NOT 把全部 `mcp:…` 抄进 Available tools 散文
- MUST NOT 插假 session system/user「MCP ready」

自定义 SYSTEM.md / `custom_prompt` 整段替换语义保持 pt9（不偷偷回填默认清单）。

## 已钉 Open Questions

1. **overlay**：`ToolSet::overlay_by_name` + Driver/composition 单一 `rebuild_agent_tools`（推荐）。
2. **二次 reload UX**：本波只保证进程替换；交互锁 defer c1205。
3. **agent busy bang**：保持今日行为（与 connecting 无关）。

## 非目标

- c1220；假 MCP ready 消息；改 wire；Trust UI
