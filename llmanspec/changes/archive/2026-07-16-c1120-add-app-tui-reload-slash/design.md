# Design — c1120-add-app-tui-reload-slash

## 编排顺序（建议）

```text
/reload (idle)
  → reload_keybindings
  → reload_skills (+ set_dollar_skill_catalog)
  → mcp_session.reload(servers from current AppConfig / composition)
  → reload_themes (若有产品接线)
  → reload_prompt_context
  → 系统块：逐项 ok / fail + 诊断摘要
```

任一步失败：**继续后续步骤**（部分成功），总报告含失败项；MUST NOT 因一项失败回滚历史。

## Busy

busy 时拒绝：`system` 提示「agent busy — /reload refused」类；MUST NOT 调用任何 reload API。

## MCP

复用 c1080 `McpSession::reload` / composition 缝；配置仍来自 YAML `mcp_servers`（含 `headers`）。本 change **不**改传输协议，只接线 slash。
