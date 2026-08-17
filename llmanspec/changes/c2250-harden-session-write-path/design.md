# 设计：session 写入路径加固

## 1. 原子重写

### 算法

`write_entries_to_disk`（`src/infra/session/manager.rs:133`）改为：

1. 序列化全部条目为 String（现状不变）；
2. 写 `<session_id>.jsonl.tmp`（**同目录**，同文件系统保证 rename 原子性）；
3. `File::sync_all()`（fsync 数据）：重写路径罕见（deferred flush 每会话一次、fork/header repair 更少），成本可接受；目标是崩溃不出现半文件，掉电持久性顺带增强；
4. `tokio::fs::rename` 覆盖目标；
5. 失败路径 best-effort 删除 `*.tmp`。

### 决策与边界

- **不抽公共 util**：仓库三处先例（trust / settings / tokenizer）各自内联；本票是第四处仍内联，出现第五处再考虑抽（避免为「未来复用」预挖抽象，违反扩展开闭纪律）。
- **不 fsync 目录**：与三处先例一致，不过度工程。
- **append 路径不动**：`OpenOptions::append` + 单行 `write_all` 保持；崩溃半行由 s20 skip-warn 兜底（该行为是损坏容忍，不是兼容）。
- **三条触发路径全覆盖**：(a) 新会话 deferred flush（`manager.rs:399→177`，新建文件同样走 tmp+rename）；(b) flush merge 重写既有文件（`:175`）；(c) `create()` header repair（`:286-296`）。fork 的 body 复制走 `append_with_id`（append），不经此路径，不动。
- InMemory backend 不受影响（`session_path` 返回 `/dev/null` 且不触发写）。

## 2. mutator async 化

### 根因与改法

`select_model_with_source` / `persist_thinking_level_change`（`src/agent/capabilities/model_ops.rs:73-103,140-158`）因同步 `&mut self` 签名不能 await 才用 `tokio::spawn`。改 async 后直接 await `store.append_session_entry`，同一 store `Arc` 上顺序 await → 排序确定。

### 转发链（自内向外）

| 层 | 位置 | 改动 |
|---|---|---|
| capabilities | `model_ops.rs:58,66,128,133` | 四方法 → `async fn`；删 spawn；失败 `log::warn!`（target `xylitol::session`，带 kind） |
| runtime 转发 | `src/agent/runtime/react/mod.rs:263-283` | 跟进 async |
| driver trait | `src/app/core/driver/proto.rs:49,57,66` | `select_model` / `set_thinking_level` / `cycle_thinking_level` → `async fn`（trait 已 `#[async_trait]`，`proto.rs:27`，无新模式） |
| in_process | `src/app/core/driver/in_process.rs:537-605` | 跟进 |
| remote | `src/app/core/driver/remote.rs:295-384` | RPC 客户端本在 async 上下文，仅签名跟进 |
| harness | `src/app/tui/harness.rs:503-539` | 跟进 |
| 调用点 | `src/app/core/dispatch.rs:127,139`、`src/app/core/bootstrap.rs:723,742`、`src/app/tui/effects/pending_ui.rs:215,223` | 补 `.await` |
| 测试 | `rg '\bselect_model\(|set_thinking_level\(|cycle_thinking_level\(' src/` 约 40 处 | 补 `.await`（多数在 async 测试或需改 `#[tokio::test]`） |

### 错误处理语义

- append 失败：`log::warn!`，**不**返回 `Err` 阻断模型切换——落盘失败不应让 `/model` 操作失败（内存态已切换成功）；要不要升级为用户可见 `XyEvent` 是独立产品决定，本票不做。
- `support.rs:109`（`persist_agent_message_with_thought_elapsed` 内 `let _ =`）：同样改 `log::warn!`。

## 3. s7 改写文本（Specs landing 用）

产品级措辞（不绑实现细节；替换 `llmanspec/specs/agent-session-store/spec.toon` 中 s7 原句「SessionManager MUST 使用原子 append（仅追加 JSONL）与文件锁保证并发安全。」）：

```toon
s7,写入安全,"SessionManager 写会话文件 MUST 满足崩溃原子性：进程任意时点终止后，盘上会话文件要么保持旧完整内容、要么呈现新完整内容，MUST NOT 出现截断或半行混合状态。系统以单进程单写者为并发假设（同一会话至多一个进程写入），MUST NOT 声称提供跨进程文件锁互斥。"
```

- s12（延迟持久化）、s20（坏行 skip-warn）等其余条款不动。
- scenarios：若现有 s7 场景行引用旧语义一并更新；建议补一行非执行场景（feature=false）：给定存在既有会话文件，当 flush merge 重写过程中进程终止，则盘上文件仍为旧完整内容或新完整内容。

## 4. 测试设计

- **原子性结构测试**：mock 失败（tmp 写失败 / rename 失败）→ 断言原文件字节不变、无 `*.tmp` 残留；成功路径 → 内容正确、无 `*.tmp`。不模拟真实崩溃（结构上 rename 保证语义）。
- **排序测试**：`select_model("m")` → 立即 `run` 一轮 → 读 JSONL：modelChange 行出现在该轮 assistant 行之前。
- **既有测试迁移**：async 化后约 40 处调用补 `.await`，同步测试上下文改 `#[tokio::test]` 或 `block_on`（跟随仓库既有测试习惯）。
