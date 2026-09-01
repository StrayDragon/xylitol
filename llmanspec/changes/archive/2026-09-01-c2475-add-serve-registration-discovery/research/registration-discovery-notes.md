# 服务注册发现契约 外部对照笔记（c2475）

> 调研来源：外部参考实现（生产级 coding agent），2026-08-23 摘录。
> 本笔记仅作选型对照；关键结论已摘要进 proposal。

## 参考实现一手证据

**发现契约**（其 client 包 `src/effect/service.ts:19-27` 注释即契约）：

> The service daemon advertises itself through a registration file in the user's state
> directory: url, pid, version, and the private password, with 0600 permissions.
> **That file is the complete discovery contract** — reading it is all a client needs to connect.

- 路径 `$XDG_STATE_HOME/<app>/service.json`；Schema：
  `{id?, version?, url, pid(>0), password?}`；
- 原子写（temp+rename），权限 0600。

**探针与门禁**（discover/incumbent/ensure）：

- probe = 带凭据 GET `/api/health`，**校验响应 pid == 文件 pid 且 version 一致**，
  否则视为无效注册；
- ensure() 幂等拉起：健康且版本兼容 → 复用；不兼容 → terminate 在任
  （SIGTERM → 50ms×100 轮询 → SIGKILL → 清文件）→ 自己 spawn；无文件/连续探活失败 ≥3 次
  → 清陈旧注册 + 指数退避 spawn detached contender（最多并存 2 个，
  "A contender is never killed merely for slow startup"）。

**在任识别与自我驱逐**：

- `server-process.ts:56`：启动前先探测目标端口已有合法在任者 → 直接退出不抢；
  EADDRINUSE 时 recognizeIncumbent（100ms 重试 ×15s），识别成功视为正常共存；
- `services/service-registration.ts`：监听成功写注册文件后，
  **每 5s 复读校验自己仍持有该文件**（字段全等才算自己的），
  被替换 → shutdown 自杀——防僵尸 daemon 与双 daemon 脑裂。

**端口约定**（service-config.ts:33-37）：主渠道固定 `0xc0de`(49374)；
其它渠道按渠道名 hash 稳定选口。多客户端共享同一 daemon：显式 `--server` /
managed ensure / standalone 三来源。

## 设计动机

- 「谁是 daemon」的仲裁完全落在文件系统 + 进程存活，无需锁服务或 leader 协议；
- 客户端零握手接入：读一个文件就知道 url/凭据/版本；
- 版本门禁使升级时旧 daemon 自动换血；owns 自检使被顶替者自行退场。

## xylitol 现状核对（2026-08-23）

- 固定 `127.0.0.1:18790`；TUI attach 未在听即失败（产品模型如此，保留）；
- `src/app/server/runtime.rs:215` 有 EADDRINUSE 检测但只报错，无法区分
  「本服务旧实例 / 无关进程占口」；
- 无任何 state 注册文件；attach 失败信息不可操作；无 pid/version 门禁。

## 落点判断

- 不引入 ensure/contender 选举（是否自动拉起是独立产品决策）；只补三件事：
  serve 写注册文件、daemon owns 自检自驱逐、attach 失败路径读文件给可操作错误。
- healthz 响应扩展 `{pid, version}` 供校验；password 字段留给 c2485。
