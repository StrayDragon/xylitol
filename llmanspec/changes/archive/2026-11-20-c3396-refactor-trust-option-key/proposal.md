---
depends_on: []
title: Trust 选项按稳定 key 选择，展示文案不再承载行为
needs_specs_change: false
branch: sdd/c3396-refactor-trust-option-key
base_branch: main
base_sha: 82b315aeef8752d86b7a4612d50eea35161e66d2
---

## Why

信任门禁（trust gate）在项目未信任时经 ChoicePrompt 让用户选择是否信任，选中的选项随后被写入 trust store。当前行为选择的判定依赖选项的**展示文案**：

- driver 侧按 `label.starts_with("Trust parent")` 与 `label == "Do not trust"` 匹配（`crates/xylitol-runtime/src/driver/in_process/mod.rs:485`）。
- 文案是呈现层资产：产品 TUI 自己的文案是 "Do not trust this project"（`crates/xylitol-app-tui/src/layout/root/mod.rs:381`），与 driver 匹配串不同，证明 label 不是身份。
- 选项结构体已带稳定标识字段 `key`（"trust" / "trust_parent" / "trust_session" / "deny" / "deny_session"，`crates/xylitol-runtime/src/trust/store.rs:471`），且答案解析 `resolve_trust_answer` 正是按 key 解析（`store.rs:36`）。行为选择却弃用了它。

后果是：改一处用户可见文案即可能让持久化路径失效；文案匹配把「呈现」与「行为」耦合，rust-analyzer 无法从 key 跳到行为。

同一「选中选项 → 落盘」逻辑存在**三份**实现（提案原称两份，深挖更正）：

- 门禁路径 `resolve_project_trusted`（`crates/xylitol-runtime/src/trust/resolve.rs:76-85`）：`let _ = manager.apply_updates(...)` **吞错**。
- driver 路径 `apply_trust_option`（`crates/xylitol-runtime/src/driver/in_process/mod.rs:503-522`）：`mgr.apply_updates(...)?` **报错**。
- CLI 路径 `src/cli/trust_gate.rs:82-107`：`apply_updates(...).map_err(|e| TrustGateError::Other(format!("trust store write failed: {e}")))?` **报错且换文案**。

三处分支逻辑相同（有 updates → apply_updates，否则 → record_decision session_only），但错误语义各不同。局部性缺失：行为规则分居三处，且 CLI 那份提案完全未提及。

另有两处**行为不等价**需在收敛时处理：

- `ProjectTrustMode::TrustCwd` 当前取 `options.first()`（`in_process/mod.rs:491`），**不校验 key**。若选项集顺序变化会静默选错；按 key 映射到 `"trust"` 后此风险消失，属**行为收紧**（正常情况下可观察结果相同）。
- `ProjectTrustMode::TrustParent` 在无父目录时 `get_trust_options` 不生成 `trust_parent` 选项（`store.rs:487-506` 的 `if let Some(ref parent)`），按 key 查找返回 None，错误文案仍为 "no parent folder to trust"——**等价**，既有测试 `trust_parent_at_filesystem_root_errors`（`in_process/mod.rs:1155`）继续守护。

## What Changes

- Trust 域内新增按行为意图选择并应用选项的单一入口，取代按展示文案查找选项。
- 行为选择不再读取 `label`；文案改动不影响持久化语义。
- 门禁路径、driver 路径与 CLI 路径（`src/cli/trust_gate.rs`）共用同一「选项 → 落盘 / 仅会话」实现，三份收敛为一处。
- `get_trust_options` 与 `TrustOption.label` 保留，仅承担呈现职责（ChoicePrompt 展示、stdio 数字菜单）。
- 错误语义保持不变：无父目录可信任时仍报 "no parent folder to trust"；其余不可用时报 "trust option unavailable"。
- `ProjectTrustMode`、`ProjectTrustPersistReport` 的公开形状与文案不变；app-tui `/trust` 命令（经 `persist_project_trust`）行为不变。

产品可观察行为不变：用户在 ChoicePrompt 中选中任一选项后落盘结果、审计事件、会话内生效范围与现状一致。因此本 change 走实现层路径（refactor），不改行为合约；但既有合约条款 `domain-security` r74 / r73 与 `app-tui-trust` r1375 的「提交须经 TrustManager 写入持久 store」在本 change 后由同一入口承担，需要在 tasks 中以既有 BDD 场景回归守护。

## Capabilities

- `domain-security`：信任解析管线与持久化（r73、r74、r76）——行为不变，实现入口收敛。
- `app-tui-trust`：ChoicePrompt 答案按稳定 key 解析（r3307）与提交经 TrustManager（r1375）——行为不变。

## Impact

- 受影响 crate：`xylitol-runtime`（trust 域 + driver in_process）与根包 `xylitol`（`src/cli/trust_gate.rs` 收敛到同一入口，错误文案映射保留）。
- 不触碰 `xylitol-app-tui`、`xylitol-kernel`、`xylitol-serve`、`xylitol-wire`、`xylitol-protocol`。
- 无线协议、无配置 schema、无持久化格式变更（trust store 文件格式不变）。
- 无破坏性合约变更，不需要迁移路径。

## Open Questions

### 深挖新发现（2026-11-20）

- **CLI 第三份实现**：`src/cli/trust_gate.rs:82-107` 与另两处逻辑相同但错误语义不同。提案原「收敛两份」须改为「收敛三份」。
- **`TrustCwd` 按 key 后行为收紧**：现状 `options.first()` 不校验 key；改为 key 查找是等价路径上的加固，不改变正常可观察结果。
- **r3307 合约漂移**：`app-tui-trust.feature:44`（`@req:r3307`）明文「trust 门禁 须按稳定 key 解析 ChoicePrompt 答案」。门禁路径已按 key；**driver 路径按 label，与该条款精神不符**。这不只是实现细节，是现有条款未覆盖 driver 侧的合约缺口。

### 深挖（逐问深挖分支）已解决的问题：

1. **是否改行为合约？** 否。用户可见行为（选项集、落盘结果、审计事件、错误文案、会话内生效）全部不变，仅内部选择依据从 label 改为 key。因此 `needs_specs_change: false`，不编辑 live `.feature`。
2. **新入口放在哪一层？** `xylitol-runtime` 的 trust 域（与 `TrustManager` 同 crate），不新增 crate、不改依赖方向；driver 只负责取 cwd / agent_dir / 审计上下文并调用。**深挖补正**：`ProjectTrustMode` 是 `crates/xylitol-runtime/src/driver/types.rs:24` 的 driver 词表，映射若放 trust 域会让 trust 域认识 driver 概念（trust → driver 反向依赖）。映射宜放 driver 侧，trust 域只提供按 `&str` key 的纯查找与应用。
3. **`ProjectTrustMode` 是否保留？** 保留。它是 app-tui `/trust` 命令与 Driver 接口（`proto.rs:242`）的既有词表，本 change 只把它映射到 key 而非 label。
4. **`TrustOption.label` 是否删除？** 不删。ChoicePrompt 与 stdio 菜单仍需展示文案；本 change 只禁止行为路径读取 label。
5. **是否顺手统一 `resolve_project_trusted` 与 driver 的重复逻辑？** 是。两者都在同一 crate 内，收敛到单一入口可提升局部性，且不改变任何外部行为。**深挖补正**：CLI 路径（`src/cli/trust_gate.rs`）是第三份，也应收敛；但 CLI 的错误文案（`trust store write failed: {e}`）须保留，收敛入口须返回 `Result` 让调用方各自映射。
6. **测试 seam 用什么？** 复用既有 harness：`xylitol-runtime` 内联单测（`in_process/mod.rs` 的 trust 测试组与 `store.rs` 测试）加既有 BDD 场景回归（`domain-security` r73/r74 与 `app-tui-trust` r3307）。不新建 harness seam。
