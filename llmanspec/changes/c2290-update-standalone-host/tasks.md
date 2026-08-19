# Tasks

测试边界见 `design.md` §6。每个切片切穿合约 → 实现 → 对应 `.feature` / 现有 BDD。

## 1. 合约与后续票暂停

- [ ] 1.1 `llman sdd change start c2290-update-standalone-host`（或已在 feature 分支则 `attach`）
- [ ] 1.2 live specs：`layer-architecture` / `app-tui` / `cli-entry` / `protocol-app` / `server-core`（见本 change Specs landing）
- [ ] 1.3 c2302 / c2303 `depends_on` 含本票；c2302 proposal 注明暂停并改传输前提
- [ ] 1.4 [blocked-by: 1.2] `llman sdd validate c2290-update-standalone-host --strict --no-interactive --no-check`；commit Specs landing

## 2. 文档

- [ ] 2.1 根 `AGENTS.md`、`src/AGENTS.md`：产品 TUI = RPC client；host 角色仍 ≠ listener 定义；默认拓扑是独立 Host
- [ ] 2.2 `docs/architecture/库与多客户端.md`：薄客户端已接线为 attach；库 embed / print 仍 InProcess

## 3. Host RPC

- [ ] 3.1 [blocked-by: 1.4] HTTP JSON-RPC unary（单端点 POST）；方法进同一 dispatch；产品 REST run/abort/tree 不再为产品 TUI 服务
- [ ] 3.2 [blocked-by: 3.1] 下行 SSE 通知（session 订阅 / last_seq / resync）；反向审批走下行 request + POST 应答
- [ ] 3.3 [blocked-by: 3.1] 默认 `127.0.0.1:18790`；`server` 进 default features；占用与停机可仍用现行锁直到 c2302 重写，但产品路径已是 RPC

## 4. 产品 TUI 只 attach

- [ ] 4.1 [blocked-by: 3.2] 产品 TUI 经 RPC 客户端驱动；默认 attach 地址；未在听失败并提示
- [ ] 4.2 [blocked-by: 4.1] 流式 token / abort / 审批 UX 不回退；相关不焊在单一 socket 任务
- [ ] 4.3 print 与 `xylitol::embed` 仍 InProcess；ce16 / tui2 场景按新语义改

## 5. 校验

- [ ] 5.1 [blocked-by: 4.3] `llman sdd validate c2290-update-standalone-host --strict --no-interactive`
- [ ] 5.2 [blocked-by: 5.1] `just qa`（或 `cargo test --test bdd` + 仓库闸）
