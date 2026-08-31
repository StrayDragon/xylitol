# Design — c2475 Serve 注册文件发现契约：pid/version 门禁与自我驱逐

## 现状核对（代码事实）

- serve 端：`subcommand.rs::run` → `runtime::start(config)`（c2465 后 bind-first → 装配 → ready）。
  停机路径：`RunningServer::shutdown/Drop` + `shutdown_signal`（SIGTERM/SIGINT）。
- attach 端：`app/core/attach.rs`——`probe_host`（TCP 探活，400ms 超时）失败即
  `AttachError::NotListening` → `attach_fail_message`（固定文案），**无法区分
  没起过 / 僵死 / 端口被占**。
- healthz：c2465 四态 body（starting/ok/stopping/failed），**无 pid/version**。
- 目录惯例：数据/运行态在 `~/.xylitol/`（sessions、logs；`DefaultResourceLoader::default_agent_dir`），
  配置 SSOT 在 `~/.config/xylitol/`（`XYLITOL_CONFIG_DIR` 可覆写）。版本号先例：
  `env!("CARGO_PKG_VERSION")`（oapi.rs）。

## 决策

- **D1 注册文件**：`~/.xylitol/serve.json`（数据目录惯例；单文件，最后写入者赢），
  内容 `{"url","pid","version"}`，**原子写**（tmp + rename）、权限 0600。
  `serve` 装配成功（ready）后写入；**优雅停机/退出时删除**；SIGKILL 残留由 attach 侧
  pid 校验兜底（见 D3）。
- **D2 自我驱逐**：serve 进程内 tokio 任务每 5s 复读注册文件，字段与自身
  `{url,pid,version}` 不全等（被新 daemon 顶替 / 被删）即触发自身优雅停机退出——
  防僵尸，不需要锁文件（sr7 一致）。
- **D3 attach 诊断链**（`probe_host` 失败路径读取注册文件）：
  - 文件不存在 → 「未启动」（现状文案语义）；
  - 文件存在 → 先 GET `/healthz`：通且 body pid == 文件 pid → 理论不可达（TCP 已失败）；
    healthz 通但 pid ≠ 文件 pid 或 version ≠ 本进程版本 → 「旧版本 / 僵死注册」可操作报错；
    端口通但 healthz 非 xylitol 形态（无 pid/version 字段）→ 「端口被无关进程占用」；
    healthz 也连不上 → 「pid=<文件pid> 的 serve 已退出，重新 xylitol serve」。
  - 正常路径（TCP 通）**零开销**：不读文件。
- **D4 healthz 扩展**：所有 phase 的 healthz body 统一携带
  `"pid": <std::process::id()>` 与 `"version": env!("CARGO_PKG_VERSION")`
  （客户端判定「是不是本服务 / 是不是同版本」的依据；starting/failed 也带——
  同进程常量）。
- **D5 specs landing**：并入 `server-core.feature`——新增 `@req:sr-reg1 @human`
  （注册文件契约：ready 后原子写 0600、退出删除、5s 自检自我驱逐、healthz 携带
  pid/version、attach 失败路径可操作诊断）+ 3 条 `@executable`（ready 写文件可读 /
  attach 对僵死注册给可操作报错 / 顶替后旧 daemon 自行退出）。

## 非目标（沿 proposal）

自动 ensure / 选举拉起；「TUI 未在听即失败」默认语义；password 字段（c2485）。
