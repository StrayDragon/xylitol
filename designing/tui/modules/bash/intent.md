# bash

`!` / `!!` 执行进 scrollback：与 tool/diff 同一套 **1-cell 轨**。`!` 时 editor 边框切 success。

- idle Enter `!cmd` 进上下文；`!!cmd` 排除上下文。空 `!`/`!!` 提示不执行。
- 长输出默认折叠展示尾部，可展开；硬截断则禁展开。禁止 `ASSISTANT>` 长标签、禁止默认整行洗底。
- agent busy 时 `!` 当普通文本/steer，不另开 bash。交互 bang 仍在跑时再提交硬拒绝。
- Ctrl+G：TTY 真 `$EDITOR`；未配置则错误行，不默默 nano。
