---
depends_on: []
branch: sdd/c2475-add-serve-registration-discovery
base_sha: 50095ac9a0b48d08e133dc07fd0805362ad18e7c
checkpointed: false
---

# Serve 注册文件发现契约：pid/version 门禁与自我驱逐

## Why

产品 TUI 默认 attach 固定端口，失败时只有「未在听」一种笼统结果——无法区分
**没起过 / 起了但是旧版本 / 端口被无关进程占用 / daemon 僵死**，排障要靠人肉 ps。
外部成熟 daemon 的通行解法是用一个 state 目录 JSON 注册文件作为完整发现契约
（url/pid/version/凭据，0600），配合探针 pid 校验、在任识别与自我驱逐，
把「谁是 daemon」的仲裁完全落在文件系统 + 进程存活上，任何客户端零握手接入
（外部对照见 research 笔记）。xylitol 保持「显式 serve」产品模型不变，
仅补齐这份可观察性契约即可获得大部分收益。

## What Changes

- `serve` 启动成功后原子写注册文件：`{url, pid, version}`（0600，state 目录）。
- attach 侧失败路径读取注册文件并给出可操作错误：
  - 文件不存在 → 未启动；
  - 探活响应 pid ≠ 文件 pid 或 version 不匹配 → 指出旧版本/僵死注册；
  - 端口被占且非本服务 → 如实报告。
- daemon 每 5s 复读确认自己仍持有注册文件（字段全等），被顶替即自行退出，防僵尸。
- 健康探针响应携带 `{pid, version}` 供上述校验。

## 非目标

- 不做自动 ensure / contender 选举拉起（是否自动拉起是独立产品决策，另行立项）。
- 不改「TUI 未在听即失败」的默认语义；只让失败信息可操作。
- password 字段归 c2485 凭据门禁票扩展。

## Impact

- `src/app/server/runtime.rs` / `subcommand.rs`：注册文件写入与自检循环。
- `src/app/server/oapi.rs`：healthz 响应形状扩展。
- TUI attach 错误文案分支；BDD 补「旧版本注册可识别」场景。

## 决策记录

- 2026-09-01（ff 深挖定案，全部设计层）：注册文件定 `~/.xylitol/serve.json`
  （数据目录惯例，单文件最后写入者赢 + 5s 自检自我驱逐 = 无锁防僵尸）；
  原子写 0600，优雅退出删除，SIGKILL 残留由 attach 侧 pid 校验兜底；
  healthz **全 phase** body 统一携带 `pid`（`std::process::id()`）与 `version`
  （`CARGO_PKG_VERSION`）——attach 以此判定「是否本服务 / 是否同版本」；
  attach 诊断链仅在 TCP 失败时读文件（正常路径零开销）。详见 `design.md`。

## Further Notes

- 一手对照（外部实现的 discover/incumbent/ensure、注册 owns 自检摘录）：[research/registration-discovery-notes.md](./research/registration-discovery-notes.md)
