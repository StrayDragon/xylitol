# Tasks: c1210-update-mcp-hot-merge-ungate

## 测试缝（已对齐）

- Host / harness Enter：connecting 时 prompt 与 bang **可**进入 pending/`run`
- `ToolSet` / Driver：二次 rebuild **无**重复 tool 名
- reload/settle：旧 manager shutdown 后再装新（能测的 public/Driver 缝）
- `build_system_prompt`：默认路径 Available tools 无 `mcp:`；含 discover 一句
- `/mcp`：任意态开面板；快照含 armed；短 cue 固定 `mcp pending (see /mcp)`

## 1. Specs + 绑定

- [x] 1.1 改写 live `infra-mcp` mcp7、`app-tui-host` ath23、`agent-prompt`（builtins-only + MCP discover）
- [x] 1.2 对应 `*.feature`（删/改 gated 场景 → allowed-while-connecting；overlay / prompt 场景）
- [x] 1.3 `llman sdd validate c1210-… --strict --no-check`
- [x] 1.4 `llman sdd change start c1210-update-mcp-hot-merge-ungate` → `sdd/c1210-…`
- [x] 1.5 升格 `/mcp`：`atm17` / `ath27` / mcp7 armed 快照；feature 场景；design/proposal 同波 MUST

## 2. ToolSet + settle/reload

- [ ] 2.1 `overlay_by_name` + `rebuild_agent_tools`；禁止 settle 路径裸 extend
- [ ] 2.2 settle / reload MCP 成功：shutdown 旧 manager → set_tools(rebuild)；单测无重复名
- [ ] 2.3 关停失败可感（log/diagnostics）；不静默双活（尽力）
- [ ] 2.4 Driver/composition 快照暴露 per-server 或汇总 **tools armed**

## 3. Host 解闸 + slash 表

- [ ] 3.1 移除 connecting 对 prompt/bang 的闸；保留 busy 闸
- [ ] 3.2 删除或掏空 `when_mcp_connecting`；`/reload` connecting 可走
- [ ] 3.3 harness：connecting 可提交；旧「拒 prompt」用例改写

## 4. System 散文

- [ ] 4.1 默认 Available tools 过滤 `mcp:`；注入 MCP discover 一句（可提 `/mcp`）
- [ ] 4.2 单测：set_tools 含 mcp 后 system 散文无 mcp 枚举；tools 请求侧仍含 mcp（既有 generate 路径）

## 5. `/mcp` 面板 + 短 cue

- [ ] 5.1 解析 `/mcp`（及可选 `/mcps`）；BusySlashPolicy Allow OpenMcp；SlashCommandSource 列出
- [ ] 5.2 editor 槽面板：id · 连接态 · armed · 可选 tool 数 + 汇总；Esc 关且不 abort agent
- [ ] 5.3 可选短 cue：固定 `mcp pending (see /mcp)`；全部 armed 后收起；MUST NOT 分数/id dump
- [ ] 5.4 harness：任意态可开；armed 行可观测；短 cue 固定文案 + 反例（无 id 堆）

## 6. 校验

- [ ] 6.1 相关 `cargo test` + `validate --strict`
- [ ] 6.2 无配置 zero-cost / 头卡 connecting 进度不回归
