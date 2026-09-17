# Tasks — 全仓 @req 前缀机械迁移为 r<数字>

## 测试 seam

- 无代码测试：这是 spec 元数据迁移，验收 = `llman-sdd validate --specs` 全绿
  + 唯一性/残留断言脚本。
- BDD bindings 按 `path + name` 绑定，场景名不变即不受影响；用
  `cargo test --lib tests::bdd::agent_session_store` 抽查 1 个 capability 确保
  runner 未断。

## 实施任务

- [x] T1 写迁移脚本：扫描 64 个 `.feature`，分配唯一 `r<1000+N>`（已合规
  `r\d+` 保留），输出 per-file 映射 + 生成 `req-id-migration-map.md`。
- [x] T2 执行迁移：逐文件替换 tag；更新 `src/app/mod.rs` 注释 `@req:tt08`。
- [x] T3 校验门禁：`llman-sdd validate --specs --no-check` 全绿；脚本断言
  （新旧 1:1、全局唯一、无残留非 r 前缀）；抽查 BDD 场景通过。
- [x] T4 收尾：`git diff --stat` 仅 tag 行变化；commit 并 finalize 归档。
