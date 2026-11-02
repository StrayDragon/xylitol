# Tasks

## 1. 合约

- [x] 1.1 `cli-entry`：改写 r1391 为本机默认口确保 Host；显式非默认 `--attach` 仍 fail-closed
- [x] 1.2 `server-core`：last-client / persistent、idle grace、注册可选 `lifecycle`、`serve stop` 真停、EADDRINUSE 确保重试
- [x] 1.3 `app-tui-preview`：r2906 designing 与产品 TUI 同一确保语义
- [x] 1.4 `runtime-config`：`serve.lifecycle` / `serve.idle_timeout`（`false` = 自动拉起 persistent）
- [x] 1.5 `user-experience`：确保失败诊断（拉起失败 / 就绪超时），不再只提示先手 serve
- [x] 1.6 绑定分支后提交 specs（`llman-sdd change start` 再改 `.feature`）

## 2. Host 生命周期（不经 TUI 可测）

- [x] 2.1 Registration 可选 `lifecycle`；缺省字段视为 persistent；自检相等性含新字段
- [x] 2.2 WS 连接计数接到 `register_unbound_mux` / `prune_mux`
- [x] 2.3 last-client：已认证 WS 1→0 启动 idle，从未连接不 idle，新连接取消，到期优雅退出并删注册
- [x] 2.4 persistent：0 连接不退出
- [x] 2.5 `xylitol serve --lifecycle` / `--idle-timeout`；显式 serve 缺省 persistent
- [x] 2.6 `xylitol serve stop` 对注册 pid SIGTERM

## 3. Ensure 缝

- [x] 3.1 kernel `ensure_host`：探活 / 清僵死 / spawn `current_exe()` / 等 healthz 200 / EADDRINUSE 改 attach
- [x] 3.2 spawn 脱离 TTY（POSIX setsid；Windows 脱离控制台）；stdin 置空
- [x] 3.3 版本不符、非 xylitol 占用、401 无凭据：不 spawn，既有诊断
- [x] 3.4 显式非默认 `--attach`：不走 ensure
- [x] 3.5 `server` feature 关闭：保持 fail-closed

## 4. 产品入口与配置

- [x] 4.1 根包 TUI / designing 入口：ensure 成功后再 `run_product_tui_attached`
- [x] 4.2 配置 `serve` 块进 schema / example；自动拉起读取；`idle_timeout: false` 即本机 C
- [x] 4.3 根 `AGENTS.md`、`src/AGENTS.md`、`README`：开箱改为 `xylitol` 即可；显式 serve 仍常驻
- [x] 4.4 可选 `--no-ensure-host`（脚本要旧 fail-closed 时）

## 5. 验证

- [x] 5.1 BDD 绑定新/改场景（ensure、last-client 自毁、persistent 存活、stop、显式 attach 不拉起）
- [x] 5.2 occupancy / attach 单测覆盖计数与分类
- [x] 5.3 `just qa` 绿
