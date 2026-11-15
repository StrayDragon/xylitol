# Tasks

## 1. CLI 外部子命令
- [x] clap 增加 external_subcommand 兜底
- [x] 内置名优先，外部名不得遮蔽
- [x] `xylitol <name> ...` 参数原样传递，不经 shell

## 2. 发现与清单
- [x] 扫 `~/.xylitol/plugins/bin/xylitol-<name>`
- [x] 配置 `plugins.paths` / `path_from_env`
- [x] 调 `--describe`，超时与失败告警
- [x] `xylitol plugins list` / `describe`

## 3. Host 交接
- [x] `requires_host` 为真时 ensure Host
- [x] 传 `XYLITOL_HOST_URL` / `XYLITOL_HOST_TOKEN` / `XYLITOL_PLUGIN_NAME`
- [x] 插件不读 Host token 文件

## 4. 安全与失败隔离
- [x] 外部名不得遮蔽内置名
- [x] 无 shell 拼接
- [x] 缺失 / 超时 / 非零退出诊断
- [x] 可选 allowlist；信任提示后置

## 5. Specs 与测试
- [x] 新增 `cli-plugin-command` capability
- [x] 更新 `cli-entry` 子命令面
- [x] CLI 单测与发现单测
