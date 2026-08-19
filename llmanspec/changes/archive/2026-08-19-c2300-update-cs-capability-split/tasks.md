# Tasks

测试边界：`llman sdd validate` + 现有 `app-tui.feature` `@req:tui2` + `just qa` / `cargo test --test bdd`。不新增可执行 `.feature`，不新增 CLI 子进程 seam。

## 1. 归属合约进 live specs

- [x] 1.1 `layer-architecture`：新增 client/host 角色、embed 不要求监听器、session 原子、一写者（`feature: false` 审查行）
- [x] 1.2 `app-tui`：新增「TUI 只承担面本地」；**不**改 tui2/tui3

## 2. 心智文档与后续草案对齐

- [x] 2.1 根 `AGENTS.md`、`src/AGENTS.md` 写入 client/host 角色（不把 listener 写成 host）
- [x] 2.2 `docs/architecture/库与多客户端.md` 理想 vs 现状：薄客户端为产品位，attach 未接线
- [x] 2.3 后续草案对齐：c2303 允许显式 `--host 0.0.0.0`；c2302 开放项指向本票 design 的 dispatcher 缝

## 3. 校验

- [x] 3.1 `llman sdd validate c2300-update-cs-capability-split --strict --no-interactive`
- [x] 3.2 现有 BDD / 结构闸仍绿（本票无运行时改动）
