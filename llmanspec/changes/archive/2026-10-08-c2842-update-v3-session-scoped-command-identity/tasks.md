# Tasks

全部在 change 分支（当前 PR 分支）上实施；收口（finalize --into 当前分支）不列为任务。

## T1 — v3 上行透明化（command_backed 全 RAW）

- [done] `build_request`：除 `host.describe` 外全部方法走 `Request::Raw` 原文（Subscribe 亦 RAW——typed Subscribe 无 cwd 会让服务端在错误 workspace 装配写者）；v3 上行成透明信封，注入字段全程保真。
- [done] `command_to_v3` 改 `#[cfg(test)]`（保往返对拍；生产路径无死码）；wire_v3_client 测试改 RAW 断言（含 subscribe/approve_tool）。
- [done] 校验：`cargo test -p xylitol --lib wire_v3_client` 10 绿；v3/remote BDD 23 绿。

## T2 — 清理上行 typed Command 死代码

- [done] 生产路径不再调用 `command_to_v3`；服务端 `v3_to_command`/Command 解码臂**保留**（兼容旧端 / Flutter POC 仍发 typed 帧）——不删除，作为兼容解码。fbs/generated 不动。
- [done] 校验：`cargo clippy --all-features --all-targets` 0 新警告（仅既有 tests/bdd.rs 横幅文档警告）。

## T3 — get_state 反映写者模型

- [done] 实证：get_state 在写者已装配时**本就**经 writer dispatch 返回写者模型（proto.rs get_state 从 current_model 取）；T1/T2 路由修复后该语义对正确会话生效，无需新增服务端逻辑。
- [done] BDD 场景 `get-state-reflects-writer-model`（假写者会话 get_state → model==fake）与 `v3-command-keeps-session-identity`（v3 set_model 无 writer_conflict）绑定并绿。
- [done] 实机复验：`/model` 切换无 writer_conflict、错误即时表出。

## T4 — Spec 落地（server-core / cli-entry）

- [done] `server-core` 新增 `@req:r1916`（会话命令身份透明保真）/ `@req:r1917`（get_state 反映生效写者模型）+ 嵌套场景；steps/bindings 绑定。
- [done] 校验：`llman-sdd validate c2842-update-v3-session-scoped-command-identity --strict` 绿。

## T5 — 门禁与收口预备

- [done] `cargo test --all-features --test bdd` 951/952 绿（唯一既有 otel flake，P1 处理中）；clippy 无新增警告。
- [done] 实机复验通过；收口：`change finalize --into 当前分支`。
