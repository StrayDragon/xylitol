---
depends_on:
- c2710-refactor-driver-command-dispatch
branch: sdd/c2740-remove-debug-from-driver
base_sha: b2604d2845b4bdb5f8c2de881dd5073e442e473f
rules_edit_acked: true
checkpointed: false
---

# `/debug` 退出 Driver/wire；Host 泵拆分（次段）

## Why

`load_debug_scene` 占 `XyDriver`、host unary、TUI slash、debug fixture 注册表，把实验室夹具冻进产品信封。产品 TUI Host 同时泵键、画、桥接，复杂度已顶闸。发布前：debug 不得走公开 Driver/Command；Host 泵按 `src/app/tui/AGENTS.md` 入口闸拆分，不按行数乱拆 crate。

## What Changes

**主段（MUST）：**

- 删除 wire/Driver/host 上的 `load_debug_scene` / Debug Command（若有）。
- `/debug` 仅 TUI 进程内：harness / `debug_fixtures` 写 session 或走 **非产品** `cfg(test)` / `debug_assertions` API，不进 `protocol::Command`、不进远程 unary。
- 产品 slash 完成器与 registry 去掉 debug 场景（或仅 debug build）。

**次段（SHOULD，可同一 change 后半或认领时拆 PR）：**

- `src/app/tui/host`：输入泵 / 布局入口保持 ≤32/27 硬闸；超标逻辑下沉到已有 `effects`/`layout` 模块，**禁止**新 `xylitol-tui-host` crate。
- 不借拆 Host 新增 Xy*。

## 非目标

- 不删 TUI 测试夹具能力；只是不再冒充会话 RPC。
- 不重做整个 TUI 架构。

## Capabilities

start 后改 live spec：产品信封无 debug scene；`/debug` 非跨面 MUST。Host 复杂度闸已在架构文档，不必新 capability，除非 spec 钉了 `XyDriver::load_debug_scene`。

## Impact

远程 TUI 不能经 Host 加载 debug scene（预期）。本地 `just test-tui` 改走 harness 注入。

## 本批依赖

`c2710`：先有 Command SSOT 再删 debug 格，避免双表残留。建议在 `c2730` 之后做（不硬 depends，减少并行冲突）；认领时若与 c2730 同分支需协调 Driver 执行器。
