# Tasks: c1875-update-force-compact-ux

## 1. Specs landing

- [x] 1.1 修订 live `domain-compaction`：`c17` 失败串对齐 design 表（empty / keep-window / Already compacted）；`.feature` `@req:c17` 措辞同步；`llman sdd validate domain-compaction --strict --no-check`

## 2. 实现文案

- [ ] 2.1 `prepare_compaction` 返回串按 design SSOT 替换所有 `session too small`；单测 exact 断言更新
- [ ] 2.2 `PI_DELTAS.md` 记 force 失败串偏离 pi
- [ ] 2.3 相关 BDD step 若钉死旧全串则改为前缀或新串；`just test` 相关 / `cargo test -p xylitol --lib prepare_compaction` 等绿灯

## 3. 校验

- [ ] 3.1 `llman sdd validate c1875-update-force-compact-ux --strict`；确认无强制再压、无观测偷渡
