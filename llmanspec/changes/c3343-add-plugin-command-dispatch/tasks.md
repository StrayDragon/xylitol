# Tasks

## 1. CLI 外部子命令
- [ ] clap 增加 external_subcommand 兜底
- [ ] 内置名优先，外部名不得遮蔽
- [ ] `xylitol <name> ...` 参数原样传递，不经 shell

## 2. 发现与清单
- [ ] 扫 `~/.xylitol/plugins/bin/xylitol-<name>`
- [ ] 配置 `plugins.paths` / `path_from_env`
- [ ] 调 `--describe`，超时与失败告警
- [ ] `xylitol plugins list` / `describe`

## 3. Host 交接
- [ ] `requires_host` 为真时 ensure Host
- [ ] 传 `XYLITOL_HOST_URL` / `XYLITOL_HOST_TOKEN` / `XYLITOL_PLUGIN_NAME`
- [ ] 插件不读 Host token 文件

## 4. 安全与失败隔离
- [ ] 外部名不得遮蔽内置名
- [ ] 无 shell 拼接
- [ ] 缺失 / 超时 / 非零退出诊断
- [ ] 可选 allowlist 与信任提示

## 5. Specs 与测试
- [ ] 新增 `cli-plugin-command` capability
- [ ] 更新 `cli-entry` 子命令面
- [ ] CLI 单测与 BDD
