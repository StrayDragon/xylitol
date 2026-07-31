# Tasks: c1210-update-mcp-hot-merge-ungate

## 测试缝（已对齐）

- Host / harness Enter：connecting 时 prompt 与 bang **可**进入 pending/`run`
- `ToolSet` / Driver：二次 rebuild **无**重复 tool 名
- reload/settle：旧 manager shutdown 后再装新（能测的 public/Driver 缝）
- `build_system_prompt`：默认路径 Available tools 无 `mcp:`；含 discover 一句

## 1. Specs + 绑定

- [ ] 1.1 改写 live `infra-mcp` mcp7、`app-tui-host` ath23、`agent-prompt`（builtins-only + MCP discover）
- [ ] 1.2 对应 `*.feature`（删/改 gated 场景 → allowed-while-connecting；overlay / prompt 场景）
- [ ] 1.3 `llman sdd validate c1210-… --strict --no-check`
- [ ] 1.4 `llman sdd change start c1210-update-mcp-hot-merge-ungate` → `sdd/c1210-…`

## 2. ToolSet + settle/reload

- [ ] 2.1 `overlay_by_name` + `rebuild_agent_tools`；禁止 settle 路径裸 extend
- [ ] 2.2 settle / reload MCP 成功：shutdown 旧 manager → set_tools(rebuild)；单测无重复名
- [ ] 2.3 关停失败可感（log/diagnostics）；不静默双活（尽力）

## 3. Host 解闸 + slash 表

- [ ] 3.1 移除 connecting 对 prompt/bang 的闸；保留 busy 闸
- [ ] 3.2 删除或掏空 `when_mcp_connecting`；`/reload` connecting 可走
- [ ] 3.3 harness：connecting 可提交；旧「拒 prompt」用例改写

## 4. System 散文

- [ ] 4.1 默认 Available tools 过滤 `mcp:`；注入 MCP discover 一句
- [ ] 4.2 单测：set_tools 含 mcp 后 system 散文无 mcp 枚举；tools 请求侧仍含 mcp（既有 generate 路径）

## 5. 校验

- [ ] 5.1 相关 `cargo test` + `validate --strict`
- [ ] 5.2 无配置 zero-cost / 头卡 connecting 进度不回归
