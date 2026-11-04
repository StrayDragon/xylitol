# Tasks

## 1. 版本比较

- [ ] 1.1 在 workspace 增加 `semver` 依赖（若未存在）；只在 kernel 使用。
- [ ] 1.2 新增 `compare_host_version(own, peer) -> HostVersionOrder`：Older / Same / Newer / Unparseable。
- [ ] 1.3 单测：old / same / newer / unparseable / `0.0.0-pre`。

## 2. 替换路径

- [ ] 2.1 重构 `classify_running` 或新增 `inspect_running`：区分 Older / Same / Newer / NonXylitol，保留既有诊断。
- [ ] 2.2 `ensure_host_with`：Ready + Older → 替换；Ready + Newer → fail-closed；Same → attach。
- [ ] 2.3 复用 `request_exit`；等待 pid 退出 5s；超时给可操作错误，不 SIGKILL。
- [ ] 2.4 替换锁 `~/.xylitol/serve.replace.lock`；拿锁后重探活；锁忙时等新 Ready。
- [ ] 2.5 spawn `AddrInUse` 沿用忽略并等待 ready；清 stale 注册（pid 已死时）。
- [ ] 2.6 新 Host 生命周期走当前 ensure 策略；不继承旧 persistent。

## 3. 边界

- [ ] 3.1 仅 ensure 口；显式 `--attach` / `--no-ensure-host` 不替换。
- [ ] 3.2 非 xylitol 占用不杀不 spawn。
- [ ] 3.3 非 Unix：替换 fail-closed，不假装成功。
- [ ] 3.4 attach 后协议版本不符仍 fatal，不重试风暴。

## 4. 测试与规格

- [ ] 4.1 wiremock / fake spawn：Older 替换、Newer fail-closed、NonXylitol 不杀。
- [ ] 4.2 `request_exit` / 锁 / 并发测试。
- [ ] 4.3 `cli-entry` r1391 增加替换 / fail-closed / 非 xylitol / 显式 attach 场景。
- [ ] 4.4 `user-experience` r3065 增加替换失败 / 客户端过旧诊断场景。
- [ ] 4.5 手测：旧→新替换、新→旧 fail-closed、非 xylitol、`--attach`。
- [ ] 4.6 `just fmt`、`just test`、相关 clippy、`just qa`。

## 5. 收口

- [ ] 5.1 确认替换失败不会留下半死 Host / stale 注册。
- [ ] 5.2 确认旧客户端诊断可操作。
- [ ] 5.3 用户确认后再开 PR；确认前不 push。
