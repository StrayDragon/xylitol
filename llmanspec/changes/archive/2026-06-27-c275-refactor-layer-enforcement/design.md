# c275-refactor-layer-enforcement — Design

> 记录本变更的关键决策。纯结构/护栏修复，无运行时行为变化。

## 1. 为什么用"白名单豁免"而不是"一次性全修"

**决策**：arch_guard 升级为"全量扫描 + 存量白名单"，而非"扩大扫描并立即修复全部违规"。

**理由**：38 处违规分属三类性质不同的耦合（类型滞留 infra / 装配发生在 agent / exec 原语借用），
每类修法不同（上提 core / 下沉组合根 / 注入），塞进一个 change 违反 SDD atomic 原则（max 2h/task），
且任一处改错都会破坏 BDD。白名单让护栏**立即生效**（防新增），存量交给 c276/c277/c278 分批清理。
每项强制标注 follow-up change id，技术债可 grep、可追踪（呼应 HC-6）。与 `la11` 移除 rpc.rs 例外的
渐进思路一致（c260 先例）。

**代价**：白名单是"已知债务的显式清单"，需在 c276/c277/c278 完成后逐项删除。膨胀是可见的
（diff 评审能看到），远好于"守卫全绿、实则 38 处违规"的隐性状态。

## 2. auth 归位：为什么是 interactive/cli，不是 infra

`agent/auth/guidance.rs` 实质：4 个纯函数，依据 `ModelKind` 拼接**面向用户的静态提示字符串**。
依赖仅 `core::model::ModelKind`，零运行时、零 I/O、零凭证校验，唯一消费方 `interactive/cli`。

| 候选 | 判断 |
|---|---|
| 留 `agent/` | ❌ 违反"agent 薄" + `auth` 名实不符 |
| `infra/` | ⚠️ infra 是运行时域（adapter 实现），放纯文案进去名不副实、过重 |
| `interactive/cli/` | ✅ 它就是 CLI 表皮的文案；单消费方，YAGNI |

函数名保留（`user-experience` spec ux1–ux4 按名锁定）。若将来 server 形态需同类引导，再提取共享位置
（当前不做，HC-5）。

## 3. arch_guard "生产代码"判定

逐文件定位首个 `#[cfg(test)]` 行，该行之前的 `crate::infra::` 命中计为生产（进白名单），之后计为测试
（不进白名单——测试可自由装配具体类型，HC-1 针对生产耦合）。这是 c260 §0"装配在组合根发生"的
编译期近似。误判用 `// NOTE:` 标注后加入白名单。

## 4. spec 内部一致性修复（la2 vs la9）

`layer-architecture` 当前 `la2`（宽：不 import 任何 infra 具体类型）与 `la9`（窄：只查 4 provider）
矛盾。本变更 `modify la9` 把守卫范围与 `la2` 对齐，并在 statement 显式写 "MUST reconcile with la2"。

## 5. 不做的事（YAGNI）

- 不实现词汇上提（c276）、装配下沉（c277）、session 瘦身（c278）——独立 change。
- 不改 `protocol.rs` → `protocol/` 目录（la5 写 "protocol/ module"，单文件也是合法 module）。
- 不动 `user-experience` / `cli-entry` spec。
- 不重构 `agent/session/export.rs`（c278 处理）。
