---
depends_on:
  - c2957-add-loop-example-plan-state
title: 自愈循环：确定性失败后由中间件续跑修复，而不是原地重试
---

# 自愈循环：确定性失败后由中间件续跑修复，而不是原地重试

## Why

工具失败（测试红、lint 报错、build 挂掉、CI 失败）目前只是回写一条 `is_error` 结果，然后由模型自行决定下一步。常见失效：

- 模型重复同一个失败动作；
- 模型忽略失败继续往下走；
- 模型在失败后直接宣称完成。

ADPS F4 的要点是：**确定性失败信号** → 诊断 → 局部 patch → 验证 → 回滚/升级；`max_iterations` 必须硬限制；知识缺失、业务规则变化或权限超界时转人工。

现状证据：

```text
crates/xylitol-plugin-sdk/src/loop_middleware.rs
  AfterToolCtx { turn_index, tool_call_id, tool_name, parts, details, is_error }
  DecideContinueCtx { turn_index, settle_count, has_new_user_messages, assistant, history_len, compact }

crates/xylitol-runtime/src/agent/runtime/middleware.rs
  LOOP_MIDDLEWARE_CONTINUE_LIMIT = 32   ← 已有硬上限

crates/xylitol-runtime/src/agent/runtime/react/mod.rs
  decide_continue 的 Continue 会强制下一轮模型调用（stall_count 计数）
```

## What Changes（候选，promote 时逐条重估）

### 1. 失败识别

官方 `self-heal` 中间件在 `after_tool`：

- 读 `AfterToolCtx.is_error` 与 `parts` / `details`；
- 用可配置的 **failure signature**（正则/字符串）识别确定性失败，例如 `test failed`、`error[E`、`lint`；
- 把「失败签名 + 建议的修复方向 + 验证命令」写入 `details` 或追加一个文本分片，供下一轮模型看到；
- 设置内部 `pending_failure` 状态。

非确定性失败（超时、网络抖动、权限拒绝）不得进入自愈，直接交回模型或升级。

### 2. 续跑

`decide_continue`：

```text
pending_failure 且 attempt < cap → Continue
attempt >= cap → Stop（或带稳定 kind 的错误）
无 pending_failure → 不干预（Continue，保持既有语义）
```

`cap` 默认小于 core 的 32（例如 3），避免一个失败签名把整个 run 拖到硬上限。

### 3. 验证与回滚

- 修复后必须重跑同一验证命令；只有验证通过才清除 `pending_failure`；
- 回滚策略先做**提示级**（要求模型回退/说明），不做自动 VCS 回滚；
- 升级：超过 cap 或签名属于「业务/权限」类，输出稳定错误并停止。

### 4. 与 plan-as-state 的衔接

若 `c2960` 的 checkpoint 可用，失败时更新 `resume_cursor` 的 `attempt`；没有 checkpoint 时用中间件内存计数。

## Capabilities

- `agent-runtime`：`after_tool` 失败识别与 `decide_continue` 续跑的行为合约；硬上限与升级语义。
- `agent-hooks`：不替代 hooks；hooks 仍可旁路审计失败。
- `runtime-config`：failure signatures 与 cap 配置。

## Impact

- crate：`plugins/xylitol-official-plugin-self-heal`（新官方插件）或扩展现有 loop 插件。
- 配置：`self_heal.enabled` / `signatures` / `max_attempts`。
- 测试：确定性失败续跑、非确定性失败不续跑、超过 cap 停止、验证通过清除状态、无失败不改变基线。
- 文档：`src/AGENTS.md` 补「自愈只在确定性失败上启用」。

## Out of Scope

- 自动 VCS 回滚 / 事务。
- 在医疗、财务、生产数据库迁移等高风险域自动闭环。
- 让插件自行调用模型（仍由 core 调度）。
- 把自愈做成 core 内联逻辑（保持插件化）。

## Further Notes

**ROI**：中。失败续跑能显著减少「原地重试」；但签名识别与「何时不该自愈」需要谨慎，先做可配置白名单。

**开放问题**：

1. failure signature 从工具结果文本读，还是由工具在 `details` 里给出结构化错误码？后者更稳，但需要工具配合。
2. 验证命令从哪里来？建议由模型在修复时显式给出，或从 plan step 的 `verification` 读（依赖 `c2960`）。
3. 与 `c2958`（进度提醒）的交互：失败后应同时提醒更新 todo，避免进度与失败状态不一致。
