# Tasks: c1900 MCP 首条门闸 + 工具定稿

## 1. 定稿状态与指纹（agent/runtime）

- [ ] 1.1 引入会话侧「工具定稿」状态（GATING / FROZEN）与指纹类型（名序 + 内容摘要）
- [ ] 1.2 实现按 name **upsert** 合并（禁同名双行）；定稿 API：从当前 armed MCP ∪ core 生成 FROZEN 表
- [ ] 1.3 单测：upsert、指纹一致/不一致、FROZEN 后 settle 不再扩 provider 可见表

## 2. 首条生成门闸

- [ ] 2.1 ReAct/run 路径：首条（或未 FROZEN）generate 前等待 MCP settle 或超时（defaults 常量）
- [ ] 2.2 零 MCP 配置立即 FROZEN(core)；超时 → 子集 FROZEN + 可观测诊断（不自动重试）
- [ ] 2.3 单测：门闸等待、超时子集、无配置立即放行

## 3. 移除 pending-turn 热并主路径

- [ ] 3.1 composition/MCP settle：去掉「下一 turn 静默 set_tools 扩表」主路径；改为只更新 registry，扩表仅经定稿/重定稿
- [ ] 3.2 回归：既有「next turn set_tools」单测按新语义改写

## 4. Resume /reload

- [ ] 4.1 resume：比指纹；一致续冻；不一致 → 门闸/重定稿 upsert + cue
- [ ] 4.2 `/reload` idle：再次门闸 + upsert 重定稿 + cue；busy 拒绝
- [ ] 4.3 单测或 host harness 覆盖上述分支

## 5. Specs landing + UX 文案

- [ ] 5.1 改 `infra-mcp` / `app-tui-host`（ath23 等）与 `agent-prompt` pt11 引导句（定稿后 tools 列表为准）
- [ ] 5.2 可执行 `.feature` 或文档场景（Partitioned SSOT）；门闸 cue 与 chrome 词汇对齐
- [ ] 5.3 `llman sdd validate c1900-update-mcp-first-turn-tool-freeze --strict`

## 6. 双轨接线（不实现 B）

- [ ] 6.1 ContextPolicy / WirePolicy 留出「轨 A 默认 / 轨 B 声明后」钩子注释或最小枚举，避免 apply 时堵死 `c1960`
- [ ] 6.2 文档指针：research 双轨 + `c1960` 后置验证 provider
