---
depends_on: []
branch: pr/2026-10-bdd-infra-and-contracts
base_branch: main
needs_specs_change: true
base_sha: dc379efb8430dae278ffc66c7bdd7e56b3faa202
---

# v3 产品路径会话契约：prompt 会话身份保真与生效默认模型状态同步

## Why

产品 TUI 默认以 v3 二进制线协议 attach 本机 Host（c2834/c2835 双轨收尾后）。实测发现该路径存在两处同源裂缝，导致「开箱即用」的 served 交互不可信：

1. **run 脱锚挂死**：v3 上行 `prompt` 命令只承载 `message`（fbs `Prompt { message }`），丢弃调用方发送的 `session_id` / `cwd` / `model_id` / `thinking_level`。Host 无法把 run 关联到客户端已订阅的会话，回落每进程随机 UUID 的 fallback 会话（无订阅者）——事件广播进空名单，客户端永远收不到流，TUI 无打字机且永不结束（模型侧已正常生成，tufa 资源正常释放，症状完全吻合）。
2. **生效模型状态不可见**：Host 装配会话写者时用 `models.default_model`（用户显式配置）恢复模型，但全程不发任何模型同步事件；`get_state` 走独立 reader driver 恒返回 `model:null`。客户端徽标永远 NOT-SET，而对话实际跑在配置默认模型上——执行与显示背离，用户无从得知「用的是什么模型」。

## What Changes

- 客户端 v3 上行：`prompt` 改走 RAW 原文过线（与 JSON 轨 `params` 透传同构），完整保真 `session_id` / `cwd` / `model_id` / `thinking_level`，使 run 路由到客户端已订阅会话。（实现已完成并单测锁定。）
- Host 侧：会话写者装配出或订阅者绑定时，把该会话写者的**当前已解析模型**以模型同步下行（ModelSelect 事件的 `session/event`）广播给订阅者；客户端已有 apply 面会收敛徽标。
- 显示语义（spec）：区分「用户显式配置的默认模型」（如 `models.default_model`）与「仅凭 provider 环境变量映射的厂商默认」——前者应显示为当前模型，后者仍须 NOT-SET。

## Capabilities

- `server-core`：v3 上行 prompt 会话身份保真、Host 装配/绑定生效模型同步下行。
- `cli-entry`：`unset-model-display`（r1387）措辞修订。
- `protocol`/`app-core host_client`：v3 prompt RAW 路由与字段保真（实现归属）。

## Impact

- served TUI 交互恢复：事件流可达、run 正常结束、可连续对话（已实测）。
- 徽标从 NOT-SET 收敛为已配置默认模型，消除「NOT-SET 还能对话」的误导。
- 不改 fbs schema、不 bump 协议版本；RAW 过线对旧 Host 无影响（旧 Host 按无会话 id 回落，行为与现状一致）。
- 范围外（本次不做，留作后续）：v3 其它 command_backed 方法（get_state/set_model/steer 等）的会话身份保真；`get_state` 改读写者当前模型。

## Fix 已存在性

上一轮已实测修复并验证：`src/app/core/host_client/wire_v3_client.rs` 的 `build_request` 让 `prompt` 走 `Request::Raw`，回归单测已加、`cargo test -p xylitol --lib wire_v3_client` 全绿、tmux 实机连发两轮对话均正常流式渲染与结束。本 change 将其正式化并补齐 Host 侧模型状态同步与 spec 落地。
