# c1560 Tasks

## 1. 配置键

- [ ] 1.1 `AppConfig` 增加 `tui.editor_history_seed_sessions`（`u32`，缺省 `1`）；非法/负值加载失败或 clamp（实现时与现有数字字段惯例一致）
- [ ] 1.2 配置单测：缺省=1；显式 N 可读
- [ ] 1.3 `runtime-config` 增加对应 requirement（unit 覆盖即可，`feature: false` 场景可）

## 2. 种子装载（Host / Driver 只读）

- [ ] 2.1 纯 new session 启动：按 cwd + mtime 取最近 N 个其它 session → user 正文（跳过 `/…`）→ `add_to_history`（旧 session 先）
- [ ] 2.2 `--session` / resume / switch / session-new 后：仅当前 session user 正文 **替换** editor 历史
- [ ] 2.3 **MUST NOT** 改 transcript；失败（缺 session/读失败）→ 空历史，不 panic

## 3. 合约与验证

- [ ] 3.1 修订 `ati13` + 新增 seed requirement；`.feature` 场景（new seed / resume only-current / skip slash / cwd 过滤）
- [ ] 3.2 产品 TUI harness：Fake store + ↑ 召回断言
- [ ] 3.3 `llman sdd validate c1560-update-tui-editor-history-seed --strict --no-check`
- [ ] 3.4 `just fmt` + 相关 clippy / `cargo test` 触及路径

## 不做

- CLI 旗标下沉、Ctrl+C、busy slash、queue drain（其它 change）
