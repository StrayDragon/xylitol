# Design

## 结论

- proposal 三组字段的行号断言与 HostBusy 范式断言全部属实，按 What Changes 三组逐组一提交执行。
- 计数勘误（不影响拆分）：HostSession 实测 41 个字段（非约 45），struct 跨 :81-125（非 :121）；pub fn 实测 host/ 目录 190 个（非 191）。
- 组类型照 `HostBusy`（host/mod.rs:128）形状：私有字段 + 组内方法，落 host/mod.rs 内，不开新文件。

## 现状证据表

| 断言 | 实测 | 证据（文件:行号 / 命令） |
|---|---|---|
| HostSession 约 45 字段平铺（:81-121） | **41 个字段**（:82-124，struct :81-125，含 2 个 `#[cfg(test)]` 字段 :104/:106） | read crates/xylitol-app-tui/src/host/mod.rs:81-125 |
| stream-paint 组 :95/:99-100 | 属实：stream_sync_pending :95、suppress_xy_until_stream_end :99、paint_dirty :100；全 crate 引用 16+9+7 = 32 处 | `rg -c '\b<paint_dirty>\b' src/` |
| stage-name 组 :90-92 | 属实：policy :90、attempted :91、request_pending :92；引用 4+5+5 = 14 处 | 同上 |
| footer-token 组 :107-112 | 属实：gen :107、tx :108、rx :109、last_mid_turn_footer_refresh :110、run_applied_token_settlement :111、pending_settlement_estimate :112；引用 7+6+5+4+6+4 = 32 处 | 同上 |
| HostBusy（:96）范式 | 属实：字段 :96；类型 :128-132（3 个 bool）；组内方法 `is_empty` / `label` :134-150 | host/mod.rs:96 / :128-150 |
| 191 个 pub fn | **不属实**：`rg 'pub fn' src/host/` = 190（mod.rs 109 + session_ops 51 + pending 16 + editor_history 10 + link_grace 3 + stage_name 1）；含 `pub async fn` 195；含 `pub(crate)` 206 | rg 计数 |

## 删除验证

每组完成后（以 stream-paint 为例，其余两组同理）：

    rg -n 'paint_dirty|stream_sync_pending|suppress_xy_until_stream_end' crates/xylitol-app-tui/src
    # 仅剩组类型定义与 impl 内部命中；HostSession 平铺字段名 0 命中

全部完成后：

    sed -n '81,125p' crates/xylitol-app-tui/src/host/mod.rs
    # HostSession 平铺字段 41 → 32（-12 组字段 +3 组值）

## 目标形状

- 三个私有组类型，落 host/mod.rs（`HostBusy` 先例 :128 同文件，不开新文件）：
  - `StageNameState { policy: StageNamePolicy, attempted: bool, request_pending: bool }`
  - `StreamPaintState { paint_dirty: bool, stream_sync_pending: bool, suppress_xy_until_stream_end: bool }`
  - `FooterTokenState { gen: u64, tx, rx, last_mid_turn_refresh, run_applied_settlement, pending_settlement_estimate }`
- HostSession 持三个组值；平铺字段 41 → 32。
- footer-token channel 消息元组 `(u64, Option<String>, bool)`（:107-109 同一形状写三遍）收一个 type alias。

## 不变量

1. 可观察行为不变：harness / snapshot / BDD 全绿，不改任何测试断言文本；harness / tests / snapshots 属 c3400 面，本 change 只读。
2. 不拆既有文件、不动方法归属、不改方法调用时序；组内方法只是字段读写的搬运。
3. 组类型与组字段私有；跨模块（host/ 子模块）访问经组内方法或组字段可达性维持现状语义（子模块对父模块私有项的可见性不变，组类型定义在 mod.rs）。
4. `footer_token_tx` / `rx` 所有权不变：仍由 HostSession 持组值，主循环对 rx 的消费位置不动。
5. 组值初始化与 `HostSession::new` / `new_with_interaction_mode`（:153 / :160）同步；derive 按需最小（HostSession 本身无 derive）。

## 测试边界（seam）

- 既有 host harness / snapshot / 单测是行为护栏，断言不变；每组迁移后 `cargo nextest run -p xylitol-app-tui` 全绿再进下一组。
- 不新增行为性单测：本 change 是字段组织搬运，新增测试会重复 BDD / 既有 harness 场景。
- 逐组一提交（T1 / T2 / T3 各一 commit），任一组判据不绿即停在该组内修复，不带病进下一组。

## 决策记录

| 决策 | 选择 | 理由 |
|---|---|---|
| 组类型落点 | host/mod.rs 内 | What Changes「不拆文件」取最保守读法；`HostBusy` 先例 :128 同文件；Impact 的「可落 host/ 下小组件文件」留作超出租界时的备选，不采用 |
| 组内字段命名 | 保留现名（paint_dirty 等） | 每组 14-32 处引用机械迁移，改名会放大 diff 与 review 面积；组名已提供语境，不改名收益为零风险为正 |
| footer-token 元组 | 收 type alias | :107-109 同一形状三处重复；alias 不改行为 |
| rx 处理 | 仍留组字段 | 移动 rx 会触碰主循环 select 时序，违反不变量 1 |
| 任务顺序 | stage-name（14 处）→ stream-paint（32）→ footer-token（32） | 风险升序；What Changes 未固定组顺序 |
| proposal 计数勘误 | design 按实数记录（41 / 190），不改 proposal | proposal 只读；偏差不改变三组拆分与行号证据 |
