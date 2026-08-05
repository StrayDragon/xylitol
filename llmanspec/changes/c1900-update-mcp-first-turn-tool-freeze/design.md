# Design: c1900 MCP 首条门闸 + 工具定稿

## 目标

正确性优先：provider 可见 `tools[]` 与可执行 ToolSet / 历史语义对齐；cache 可降级。
开箱走 **轨 A（定稿/冻表）**；轨 B（`defer_loading`/`tool_search`）见 `c1960`，本 change 不实现。

## 状态机（轨 A）

```text
[无 MCP 配置] ──立即──► FROZEN(core only)
[有 MCP] ──首条生成请求──► GATING(等 settle / 超时)
                              │
                    ┌─────────┴─────────┐
                    ▼                   ▼
            settle 完成            超时/部分失败
                    │                   │
                    └──────► FROZEN(core ∪ armed_subset)
                                      │
                    settle 再来 ──忽略扩表──► 仍 FROZEN
                    /reload idle ──► GATING ──► FROZEN' (upsert)
                    resume 指纹一致 ──► 续 FROZEN
                    resume 指纹不一致 ──► GATING/重定稿 upsert + cue
```

- **可键入**：GATING 时用户可提交；generate **阻塞**至 FROZEN（或错误策略，本波子集放行不硬失败）。
- **upsert**：按 tool **name** 对齐；有则替换 schema/description，无则 append；**禁止**同名多行。
- **指纹**：至少覆盖「定稿后 provider 可见工具名有序列表 + 各 name 的 schema/description 摘要哈希」；与当前 armed MCP 比对。

## 与现有行为的差分

| 旧 | 新（轨 A） |
|---|---|
| MCP settle → next turn 热并 tools | settle **不**扩已 FROZEN 表 |
| ath23：未结算也可立刻跑 agent | 可提交，但 **首条 generate 门闸** |
| resume 不显式比工具集 | 指纹不一致 → 重定稿 upsert + cue |

## System prompt

保持 **pt11**：Available tools 散文 **仅 builtins**；引导改为「本会话 MCP/扩展以 **定稿后的请求 tools 列表** 为准并精确名调用」（不枚举 mcp: 名进散文）。

## 超时常量

code-first：`defaults.rs`（或 MCP 装配旁）单一超时；本波不扩 YAML。

## 观测 / cue

- 门闸等待：短 cue（如「等待 MCP…」）——文案进 TUI 词汇表或既有 chrome 约定。
- 超时子集放行 / resume 重定稿 / reload 重定稿：系统块或 status 短提示，含「可再 /reload」；**不**自动重试连接。

## 非目标

- 实现 `c1960` hosted/client tool_search
- 禁用输入框
- 用假 hosted 冒充 defer 轨

## 测试 seam

- **Driver / AgentSession 公共 API**：门闸、定稿、冻结后 settle 忽略扩表、指纹、upsert（单测为主）。
- **产品 `/reload` + resume**：现有 TUI/host harness 或 BDD（`app-tui-host` / `infra-mcp`）——改 ath23 / mcp7 冲突句后补场景。
- **不**本 change 做 Ornith live 作 MUST；lab 结论已在 research。
