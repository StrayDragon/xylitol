# Design — c2838 doc intra-doc 链接治理

## 背景

c2837 公开化后 rustdoc 扫描 28 处既有 intra-doc 缺陷（BROKEN ×18 / PRIVATE ×10），
曾以两个 crate-level allow 兜底。本 change 逐处修复后移除 allow。

## 决策

### D1：BROKEN 类 → 补正确路径（维持链接语义）
目标是 pub 可达项（c2837 已公开 agent/infra 面内）。逐处补全 `crate::` / 模块相对路径，
依据 research/findings.md 的定义定位表。其中 `[BootstrapOutput::warnings]` 经语义核对
为**真笔误**（该类型不存在；bootstrap 返回类型 `ResolvedAssembly` 含 `pub warnings`），
修正为 `[ResolvedAssembly::warnings]`——这是本 change 唯一的实质内容修正。
`[id]`/`[scene]` 为 markdown 误解析，转 `code` 字体。

### D2：PRIVATE 类 → 改文字（不扩公开面）
被链项（实现常量 `SUMMARY_PLACEHOLDER_TOKENS`/`SHELL_CACHE`、crate-internal 子模块
`status_bar`/`commands`/`render`、内部方法 `discover_with` 等）经评估无公开 API 价值
或公开会导致连锁 doc 面扩大 → 一律改 code 字体文本（`[X]` → `` `X` ``），语义保留、
无二次涟漪。

### D3：allow 移除条件
所有 28 处处理完毕且 `cargo doc` 零 warning 后移除 lib.rs 两个 allow，恢复 rustdoc
lint 默认完整性。

## 验证
`cargo doc --all-features --no-deps` 零 warning；`just qa quiet` 全绿；validate 绿。
