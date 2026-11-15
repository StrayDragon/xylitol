---
depends_on: []
title: "Add generic plugin command dispatch"
branch: sdd/c3343-add-plugin-command-dispatch
base_branch: main
base_sha: 308b68712d1235c0c7981c45553170b1afe89de8
---

## Why

xylitol 需要把外部插件二进制接入 CLI，而不是为每个 UI 或端写特例。第一个消费者是 ThoughtDAG 画布插件（`xylitol-thoughtdag`）；后续 nsh 多端与其它 surface 复用同一套发现与委派。

现状缺口：

- CLI 子命令为固定枚举，未知子命令直接报错；没有 `xylitol <name> ...` 的外部委派。
- 外进程插件只走 stdio JSON-RPC（工具 / 命令），没有「二进制子命令」形态。
- 每个 UI 若自行约定启动方式，会把发现、Host 交接、鉴权、失败隔离散落在各插件里。

## What Changes

- CLI 增加外部子命令兜底：`xylitol <name> ...` 委派给 `xylitol-<name>`；内置名优先，外部名不得遮蔽 `serve` / `tui` / `print` / `config` / `sessions` / `plugins`。
- 发现：`~/.xylitol/plugins/bin/xylitol-<name>` 优先；显式配置路径次之；PATH 查找默认关闭，须显式开启。
- 清单：插件二进制支持 `--describe`，输出 JSON（name / version / requires_host / commands / default_bind / default_port / auth）。发现时带超时调用，失败只告警。
- 命令面：`xylitol plugins list` 与 `xylitol plugins describe <name>`。
- Host 交接：`requires_host` 为真时，CLI 走与 TUI 相同的 ensure-host 路径，并向插件传 `XYLITOL_HOST_URL` / `XYLITOL_HOST_TOKEN` / `XYLITOL_PLUGIN_NAME`；插件不得自行读取 Host token 文件。
- 安全：插件目录用户自有；不得自动下载；不得经 shell 拼接执行；外部命令失败只影响本次调用。
- 失败隔离：插件缺失、`--describe` 超时、非零退出 须给明确诊断，不得静默成功。
- 测试：CLI 单测覆盖发现、遮蔽、委派、环境变量、失败诊断；BDD 覆盖 `plugins list` 与委派。

## Capabilities

- `cli-plugin-command`（新 capability）：外部插件发现、清单、委派、Host 交接与失败隔离。
- `cli-entry`：子命令面新增 `plugins`，并明确外部名不遮蔽内置名。

## Impact

`src/cli`（参数解析与委派）、config 类型（插件路径 / PATH 开关 / 允许列表）、`llmanspec/specs/` 与 BDD。

## Decisions

- PATH 查找默认关闭；插件目录即信任边界，显式 `path_from_env` 才开。
- `requires_host` 为真时每次 ensure Host；插件不自行 attach。
- allowlist 可选；默认只扫 `~/.xylitol/plugins/bin`。
- 清单用 `--describe`，不另立 `plugin.json`。
- 范围：本 change 只推进到 planned，不实现。

## Related

- 关联仓：`https://github.com/StrayDragon/xylitol-thoughtdag`（private）。
- 关联提案：`c1-add-thoughtdag-surface`（该仓，planned）。
- 设计细节：该仓 `docs/plugin-command.md`、`docs/architecture.md`。
- 该 change 只做通用命令注册与委派；ThoughtDAG 桥接、Web surface、nsh 端各自独立 change。
