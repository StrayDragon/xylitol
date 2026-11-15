# Design

## 目标

让 `xylitol` 能发现并委派 `xylitol-<name>` 外部二进制，形成通用的插件命令面；不把 Web、ThoughtDAG 或某个端的特例写进 CLI。

## CLI 入口

- clap 增加外部子命令兜底：`#[command(external_subcommand)] External(Vec<String>)`。
- 内置子命令优先；外部名不得遮蔽 `serve` / `tui` / `print` / `config` / `sessions` / `plugins`。
- `xylitol <name> ...` 把剩余参数原样交给 `xylitol-<name>`；不经过 shell 拼接。
- `xylitol plugins list` 列出已发现插件；`xylitol plugins describe <name>` 打印清单。

## 发现

- 首选 `~/.xylitol/plugins/bin/xylitol-<name>`。
- 次选配置 `plugins.paths` 中的目录。
- PATH 查找默认关闭；开启后按 `xylitol-<name>` 匹配。
- 名称 kebab-case；二进制须可执行；发现时调用 `--describe`，带短超时。
- `--describe` 失败只告警，不影响其它插件与内置命令。

## 清单

```json
{
  "name": "thoughtdag",
  "version": "0.1.0",
  "requires_host": true,
  "commands": ["ui start", "ui stop"],
  "default_bind": "0.0.0.0",
  "default_port": 18791,
  "auth": "token-cookie"
}
```

- `name` 须等于二进制后缀。
- `requires_host` 决定是否 ensure Host。
- `commands` 仅用于展示，不参与解析。

## Host 交接

- `requires_host` 为真：走与 TUI 相同的 ensure-host 路径，ready 后 exec 插件。
- 传 `XYLITOL_HOST_URL` / `XYLITOL_HOST_TOKEN` / `XYLITOL_PLUGIN_NAME`。
- 插件不得直接读 `~/.xylitol/serve.token`。
- Host 仍 `127.0.0.1`；插件 UI 的 bind 由插件自己按清单处理。

## 配置

```yaml
plugins:
  paths: []
  path_from_env: false
  allow: []
```

- `paths` 显式目录；`path_from_env` 控制 PATH 查找；`allow` 可选白名单。
- 默认空配置：只扫 `~/.xylitol/plugins/bin`。

## 安全

- 插件目录用户自有；不得自动下载、不得签名市场。
- 外部名不得遮蔽内置名。
- 执行不经 shell；参数逐个传递。
- 首次运行可选信任提示；允许列表可持久化。
- 失败隔离：缺失 / 超时 / 非零退出只影响本次调用。

## 测试

- CLI 单测：发现、遮蔽、委派、环境变量、失败诊断。
- BDD：`plugins list` 与 `xylitol <name>` 委派。
- 不启动真实插件进程；用临时目录与假二进制。

## 与 ThoughtDAG surface 的关系

- 本 change 只做通用命令注册；ThoughtDAG 桥接、Web surface、nsh 端各自独立。
- 关联仓：`https://github.com/StrayDragon/xylitol-thoughtdag`，change `c1-add-thoughtdag-surface`。
