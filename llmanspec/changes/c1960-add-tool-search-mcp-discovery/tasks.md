# Tasks: c1960-add-tool-search-mcp-discovery

> 阶段目标：Designed / pre-start（本文件 + design 齐）。**禁止**在未 `change start` 前改 live specs / 写实现代码。

## 0. Designed 门（本波）

- [x] 0.1 读 c1900 archive design/research + 现有 `ToolsMode` / freeze / WirePolicy（`pab20`）
- [x] 0.2 钉 Open Questions（含 Q-WP WirePolicy 声明分流）；写 `design.md` 决策表
- [x] 0.3 本 `tasks.md` 垂直切片；确认 Start readiness = Designed 且未分支

## 1. Branch binding + Specs landing

- [ ] 1.1 `llman sdd change start c1960-add-tool-search-mcp-discovery`（干净树 + 默认分支）
- [ ] 1.2 live specs：`package-ai-bridge` — `tool_search_wire` 在 WirePolicy（**非** ExtraPolicy）；三态出线；默认轮廓
- [ ] 1.3 live specs：`agent-runtime` / `infra-mcp` — `tools_mode=Search`：顶栏=core+meta；MCP Deferred；门闸冻顶栏；禁 settle 扩顶栏
- [ ] 1.4 Partitioned：可执行 `.feature` 或 `feature: false` 文档场景按需；轨 A 默认不变
- [ ] 1.5 commit Specs landing；`llman sdd validate c1960-add-tool-search-mcp-discovery --strict --no-interactive`；确认 `readyToImplement=true`

## 2. Wire 声明 + forge

- [ ] 2.1 bridge：`WirePolicy` 增加 `tool_search_wire`（Unsupported/Hosted/ClientFunction）；`defaults` + `for_compat` 映射；单测锁 ExtraPolicy 无 search 位
- [ ] 2.2 Assembler/adapter：按声明 forge 元工具形状；Unsupported∧Search → 可观测拒绝（或设计钉死的回退）
- [ ] 2.3 Golden：同投影 × 三态 WirePolicy → tools/item 集 diff

## 3. Registry Deferred + 检索

- [ ] 3.1 Search 轨：MCP armed → 内部 registry（不进 provider 顶栏）；顶栏 = core + meta（[+ Hosted 声明子集]）
- [ ] 3.2 默认内存 BM25 索引；`/reload` idle 重建索引且不灌 MCP 进顶栏
- [ ] 3.3 单测：Deferred 隔离、索引命中/未命中、reload 重建

## 4. tool_search 执行环

- [ ] 4.1 ClientFunction：元工具 handler → BM25 → 约定 output；按需 upsert（Q8）；禁同名双行
- [ ] 4.2 Hosted：声明为 Hosted 时走原生 item；**禁止**假 provider 冒充 Ornith hosted
- [ ] 4.3 Fake provider 单测：跨轮顶栏名表稳定；发现只增尾部；改 loaded set 的 cache-bust 语义有注释/测

## 5. 门闸 / 与轨 A 共存

- [ ] 5.1 Search 门闸：settle/超时后冻**顶栏**；复用超时/子集/cue 心智，不锁键入
- [ ] 5.2 默认 `Full` 路径回归绿（c1900 行为不回归）
- [ ] 5.3 `tools_mode=Search` ∧ mid-turn `set_tools` 仍拒绝扩顶栏

## 6. 校验与证据

- [ ] 6.1 触及单测 + 相关 BDD；Ornith **不**标 Hosted 通过
- [ ] 6.2 （可选维护）lab：真 OpenAI client/hosted 矩阵 — **不进** `just qa` 除非另接线
- [ ] 6.3 `just qa`（开 PR 前）；verify 对照 design 决策表
