---
title: "abort 轮次落盘结束墙钟并恢复 Worked for 时长"
depends_on: []
branch: sdd/c3365-add-abort-turn-end-stamp
base_branch: main
base_sha: ab9d2c79992e9436b43bb8b45cb737c0a97c7d06
---

# abort 轮次落盘结束墙钟并恢复 Worked for 时长

## Why

会话重建（resume / travel / fork）后，abort 终止的轮次没有可靠的轮次结束墙钟：
流式产生任何内容前 abort 的 turn 不落盘任何 assistant 消息，重建时钟缺末端；
同时这类轮次没有任何可收纳中间活动，信封根本不形成，Worked for 行整行缺失。
用户在 resume 后无法看到 abort 轮次实际耗时；部分内容的 abort 轮次（已有 aborted assistant 落盘）不受影响。

## What Changes

1. Runtime：turn 以 abort 终止且本轮尚无任何已落盘 assistant 消息时，落盘一条 content 为空、
   stop_reason=aborted 的 assistant 消息作为轮次结束墙钟；标记随 AgentEnd 消息集呈现，
   不进 project_for_llm 投影（沿用既有 aborted skip），不改变 AgentEnd reason 语义。
2. 产品端投影：content 为空且无 errorMessage 的 aborted assistant 不得画助手气泡或 Aborted 通知，
   仅作为轮次结束墙钟参与信封时长恢复；部分内容的 aborted assistant 维持现行投影。
3. Transcript：abort 轮次（以落盘结束标记为准）须形成信封，即使该轮无可收纳中间活动；
   标记参与分段的簇不得画簇摘要头（沿用 compaction-only 先例）；折叠视图 须为 User 加 Worked for 行，
   时长书写沿用不足 1s 写 <1s 与双端墙钟规则。

## Capabilities

- `agent-runtime`：新增 abort 轮次结束墙钟落盘规则。
- `app-tui-bridge`：新增空 aborted assistant 的 resume/live 投影约束。
- `app-tui-transcript`：扩展信封形成与 Worked for 时长恢复条款。

## Impact

- 旧会话（本 change 前落盘）的 abort 轮次仍缺结束戳，继续按「缺任一端省略时长」处理，不迁移。
- print 端与 LLM 投影不受影响：标记 content 为空且已被投影 skip。
- 测试 seam：复用既有 app-tui-transcript BDD harness（场景构建器回放）与 runtime 单测 seam，不新建 harness。

## Further Notes

- 真实会话审计（2026-11-27）：6 个会话存在无 assistant 落盘的已结束轮次
  （如 6ffb9ecc 连续 4 条 user 无任何 assistant；09f3b89c 第 1 轮无 assistant）。
  详见 research/resume-clock-audit.md。
- 相邻缺陷已先行修复（main b9224073）：resume 时钟把毫秒截断到整秒，亚秒轮次误写 1s；
  本 change 的时长书写规则与该修复一致。
