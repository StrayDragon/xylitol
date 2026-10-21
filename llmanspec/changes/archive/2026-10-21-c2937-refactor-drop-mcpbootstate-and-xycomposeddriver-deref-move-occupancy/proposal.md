---
depends_on:
  - c2936-refactor-rutis-kernel
title: 会话准备门改为瀑布事件并收口根包 occupancy
branch: sdd/c2937-refactor-drop-mcpbootstate-and-xycomposeddriver-deref-move-occupancy
base_branch: main
base_sha: 4a1977c6d53d20c9cecef14f72888e201ecaec95
---

# 会话准备门改为瀑布事件并收口根包 occupancy

## Why

`c2936-refactor-rutis-kernel`（rutis 进程内核）把装配下沉到 kernel，MCP 首轮门超时已归 tool-table，但准备编排仍是 `XyComposedDriver` 上的 `McpBootState` 五态机。这不是可插的缝：后续 skills / `tool_search`（`c1960`）无法同挂。根包 `src/wiring` 还留着 occupancy 测试胶水（kernel 不能依赖产品 TUI）。进程内驱动仍是 Deref 包装，调用可绕过端口。

准备门的产品行为要保留：人可以先打字；有 MCP 配置时真正出站须等 settle/超时；Assembling / connecting / 超时诊断仍在。变的是插入方式。resume / 切会话须重新准备，但 unary 不得堵 MCP。正确性优先于供应商 KV cache：准备结果有变时在下一用户提问前追加 Env 行（不改写既有 turn）；无变则不追加。

## What Changes

- 新增瀑布事件 `session/prepare`（wait/timeout）。mcp 插件为首个监听器。`tool_search` 不进本 change。
- 切会话 / resume / idle `/reload`：开火不 wait（unary 立刻返回，监听器开始连）。下一次 generate：wait/timeout 后再拼出站。
- 拆掉 driver 上的 `McpBootState` 编排与 `begin/poll_mcp_bootstrap` 作为准备门的职责。进度仍经 loaded-resources / Assembling 信封。
- 准备结果相对上次有变：持久化 Env 行（对齐 `session_env`），紧挨下一用户提问前，投影为 user。无变不追加。不得把整份 system 正文写成 user。不得改写既有 turn。
- 拆掉 `XyComposedDriver` 包装，调用方使用进程内 `RuntimeDriver`。
- 产品根包只留 argv / profile。CLI 直调 kernel occupancy。`in_process_client` / dual-rail / assembly fixture 下沉 `tests/`。crate 图不动。

## Out of Scope

- `c1960` tool_search / hosted defer_loading。
- 第三方插件市场 / dylib。
- 把产品跨端协议换成 rutis-bridge。
- 指纹持久化后续冻（仍是另波；本波 resume 清冻再门）。
- 允许 kernel / serve 依赖 `xylitol-app-tui`。

## Capabilities

- `infra-mcp`：准备门走 `session/prepare`；切会话开火不 wait；generate wait；Assembling 仍在。
- `agent-runtime`：首次 generate / 未冻结时走准备门；resume 清冻。
- `agent-prompt`：准备增量 Env 行（有变才追加，在下一 user 前）。
- `workspace-assembly`：根包不再持有 occupancy 胶水。
- `layer-architecture`：进程内驱动不再经 Deref 包装（组织方向，不钉类型名）。

## Impact

plugin-sdk 事件词表、runtime ReAct 出站、mcp 插件、kernel driver、根包 `src/wiring`、BDD occupancy 入口、embed 公开进程内驱动类型。

## Further Notes

依赖已归档的 `c2936-refactor-rutis-kernel`。深挖决议：删 driver 编排职责；UX 保留、缝改为事件；三件同 change；occupancy 胶水进 `tests/`；事件名 `session/prepare`；resume 开火不 wait、generate 才 wait；正确性 > KV cache；有变追加 Env 行；拆掉 `XyComposedDriver`。
