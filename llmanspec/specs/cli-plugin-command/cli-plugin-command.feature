# language: zh-CN
# purpose: 外部插件二进制的发现、清单、委派与 Host 交接。

功能: cli-plugin-command

  @req:r3358
  规则: plugin-discovery
    CLI 须发现 `~/.xylitol/plugins/bin/xylitol-<name>`；须支持配置显式路径；PATH 查找默认关闭。发现失败 不得阻断内置命令。

    场景: 插件目录优先
      假如 插件目录存在可执行的 xylitol-demo
      当 列出插件
      那么 结果含 demo 且路径来自插件目录

  @req:r3359
  规则: external-dispatch
    CLI 须在 clap 解析前识别已安装的 `xylitol-<name>` 并委派；未知名仍由 clap 拒绝；不得经 shell 拼接；内置名不得被外部名遮蔽。

    场景: 外部子命令委派
      假如 插件目录存在可执行的 xylitol-demo
      当 执行 xylitol demo ui start
      那么 委派给 xylitol-demo 且参数为 ui start

  @req:r3360
  规则: plugin-manifest
    插件二进制 须支持 `--describe` 输出 JSON；发现时带超时调用；失败只告警。

    场景: 清单解析
      假如 插件返回含 requires_host 的 JSON
      当 解析清单
      那么 requires_host 为真

  @req:r3361
  规则: host-handoff
    `requires_host` 为真时 CLI 须 ensure Host，并向插件传 `XYLITOL_HOST_URL` / `XYLITOL_HOST_TOKEN` / `XYLITOL_PLUGIN_NAME`；插件不得自行读取 Host token 文件。

    场景: 需要 Host 的插件
      假如 插件清单 requires_host 为真
      当 委派插件
      那么 子进程环境含 XYLITOL_HOST_URL 与 XYLITOL_HOST_TOKEN

  @req:r3362
  规则: plugins-subcommand
    CLI 须提供 `plugins list` 与 `plugins describe <name>`；描述须输出插件清单 JSON。

    场景: 列出插件
      当 xylitol plugins list
      那么 输出已发现插件