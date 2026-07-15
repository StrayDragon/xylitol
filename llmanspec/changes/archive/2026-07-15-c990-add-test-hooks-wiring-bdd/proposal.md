---
change_id: c990-add-test-hooks-wiring-bdd
title: "库级 Hook 缝：精选导出 XyHookBus + wiring BDD 骨架"
status: full
priority: 990
depends_on: []
author: agent
track: A
wave: hooks-wiring-bdd
domain: c995
---

# c990-add-test-hooks-wiring-bdd

## Why

c735 已有 `XyHookBus` 端口与部分热路径，但（1）未进精选库导出，嵌入方无法把 hook 当一等契约；（2）缺少以 **Driver / composition 库缝** 为 subject 的可复用中文 BDD。现有 `hooks.feature` 偏调度器直调，易假绿；叙事若绑 TUI 会把库能力误写成应用面旁路。

## Purpose

**本变更：库契约可见 + 测试骨架；不接线 c995–c998 新产品发射点。**

1. 精选 `pub use`：`XyHookBus`、`XyHookOutcome`（及 `NoopHookBus` 若需对称）。
2. `tests/features/hooks-wiring.feature`：中文场景大纲（观察 / 可取消 / 可修改）；操作名映射 **Driver/agent API**，不映射 UI 手势。
3. 至少一条已接线 smoke：`session_start` 经 `InProcessDriver`（如 `session_tree`→`ensure_session`），禁止裸 `HookDispatcher::dispatch` 冒充。
4. design 预留 c996/c998 操作行；未接线不挂失败 scenario。

## What Changes

- `src/lib.rs`（+ 文档注释 / `embed` 提及）
- `tests/features/hooks-wiring.feature` + `tests/bdd.rs` 步骤与操作字典
- capability `test-hooks-wiring`
- `design.md`：库操作字典

## Capabilities

- `test-hooks-wiring`（add）
- 触及库导出（layer / lib 面；本 change 以 tasks 落实导出）

## Out of scope

- c995–c998 热路径接线
- TUI HostSession 作为 wiring 唯一证明
- 自定义 `BuildAgentOptions.hook_bus` 字段（仍经 `hooks_config`→`HookDispatcher`；导出端口供嵌入方自实现与测试录制）
- `input` 事件、抓包 c999

## Ethics

- risk_level: low
- prohibited_actions: 用 dispatcher 直调冒充库路径；以 TUI 为唯一 hook 引用；未接线例子弄红 CI
- required_evidence: `XyHookBus` 可从 crate 根引用；wiring smoke 绿；`validate --strict` 收尾
- escalation_policy: 若 `ensure_session` 难从 Driver 触发，改用文档化的等价 Driver API 并更新 design

## Depends

- 无硬依赖（c735 已归档）
- 后续 c996/c998/c995/c997 升格时应 `depends_on` 本 change

## 验证

- `cargo test --test bdd -- --test-threads=1`
- `llman sdd validate c990-add-test-hooks-wiring-bdd --strict --no-interactive`
