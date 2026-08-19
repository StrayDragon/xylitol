# Tasks

测试边界见 `design.md` §8。每个切片切穿合约 → 实现 → 对应 `.feature` / 现有 BDD。

**禁止** attach 已有 `sdd/c2290-update-standalone-host`（旧 JSON-RPC+SSE landing）。须 `change start` 新分支。

## 1. 合约

- [x] 1.1 `llman sdd change start c2290-update-standalone-host`（工作区干净且在默认分支；旧同名分支须已删）
- [x] 1.2 live specs：`layer-architecture` / `app-tui` / `cli-entry` / `protocol-app` / `server-core`——四象限 + 产品 TUI attach；**不要**写 JSON-RPC 2.0 或产品 SSE
- [ ] 1.3 [blocked-by: 1.2] `llman sdd validate c2290-update-standalone-host --strict --no-interactive --no-check`；commit Specs landing

## 2. 文档

- [x] 2.1 根 `AGENTS.md`、`src/AGENTS.md`：产品 TUI = 四象限客户端；host 角色仍 ≠ listener 定义；默认拓扑是独立 Host
- [x] 2.2 `docs/architecture/库与多客户端.md` 与 `远程体验与线协议.md`：薄客户端已接线为 attach；信封与通道解耦；库 embed / print 仍 InProcess

## 3. 信封、方法表、typed client、specta（监听器仍可留给 c2302）

- [ ] 3.1 [blocked-by: 1.3] `protocol`：四象限类型 + `rpcId` 回显；`RpcResult` 收口既有 REST `Envelope`/`ErrorCode`（产品路径只留一套错误）
- [ ] 3.2 [blocked-by: 3.1] 方法表按 `research/method-table.md` 落地：unary 登记、`Quit` 不进 Host、审批/问卷走 respond、下行 `session/event` 包 `Event`
- [ ] 3.3 [blocked-by: 3.2] `HostClient` trait + `InProcessClient` + `HttpWsClient`；产品 TUI 只用后者；默认 attach；未在听失败并提示。c2302 未听前产品路径允许失败，**禁止** InProcess 冒充
- [ ] 3.4 流式 token / abort / 审批 UX 不回退；相关不焊在单一 socket 任务；`ip9` 废止
- [ ] 3.5 print 与 `xylitol::embed` 仍 InProcessDriver；ce16 / tui2 按新语义改
- [ ] 3.6 `server` 进 default features；默认 `127.0.0.1:18790`
- [ ] 3.7 [blocked-by: 3.2] specta：信封+payload `Type`；导出检入 `clients/typescript/bindings.ts`；`just gen-protocol-ts`（名可微调）；`scripts/check_*.py` 重生 diff 进 `just qa`

产品 REST run/abort/tree 不再为产品 TUI 服务（可 410 或删）。

## 4. 校验

- [ ] 4.1 [blocked-by: 3.7] `llman sdd validate c2290-update-standalone-host --strict --no-interactive`
- [ ] 4.2 [blocked-by: 4.1] `just qa`（或 `cargo test --test bdd` + 仓库闸）
