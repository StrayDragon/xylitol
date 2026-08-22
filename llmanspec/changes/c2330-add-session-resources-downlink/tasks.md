# Tasks

纯合约收口，零生产代码改动（实现已由 fab4f2f7 落在 main）。

## 1. Branch binding + Specs landing

- [ ] 1.1 `change start` 绑定 `sdd/c2330-add-session-resources-downlink`
- [ ] 1.2 live specs：`server-core` w1 扩列表 + 新增 sr-resource2（含 scenarios 文档行）；`app-tui-host` 新增 ath38（含 scenarios 文档行）
- [ ] 1.3 commit Specs landing；结构过闸 + `readyToImplement=true`

## 2. 对拍核验

- [ ] 2.1 按 design.md 对拍表逐条核对 spec 陈述 ↔ 实现/测试锚点
- [ ] 2.2 相关既有测试绿（remote / host / protocol 定向过滤跑）

## 3. 收口门禁

- [ ] 3.1 全量 validate（含 BDD 编译 `--check`）绿
- [ ] 3.2 若发现陈述与实现出入：以代码为准修措辞或补最小驱动级覆盖（边界见 design.md）
