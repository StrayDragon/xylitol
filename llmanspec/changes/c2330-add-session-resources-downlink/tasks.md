# Tasks

纯合约收口，零生产代码改动（实现已由 fab4f2f7 落在 main）。

## 1. Branch binding + Specs landing

- [x] 1.1 `change start` 绑定 `sdd/c2330-add-session-resources-downlink`
- [x] 1.2 live specs：`server-core` w1 扩列表 + 新增 sr-resource2（含 scenarios 文档行）；`app-tui-host` 新增 ath38（含 scenarios 文档行）
- [x] 1.3 commit Specs landing；结构过闸 + `readyToImplement=true`

## 2. 对拍核验

- [x] 2.1 按 design.md 对拍表逐条核对 spec 陈述 ↔ 实现/测试锚点
- [x] 2.2 相关既有测试绿（remote / host / protocol 定向过滤跑）
- [x] 2.3 tmux 真机集成验收（serve@18801 + TUI attach，fake provider，隔离 env）：头卡渲染一致、不卡 mcp pending、一轮 prompt transcript 完整且无 chrome 帧混入、/mcp 面板可开、干净退出

## 3. 收口门禁

- [x] 3.1 全量 validate（含 BDD 编译 `--check`）绿
- [x] 3.2 若发现陈述与实现出入：以代码为准修措辞或补最小驱动级覆盖（边界见 design.md）——对拍无出入；另发现 host 默认模型恢复用裸 model 名而非 alias 的旁支缺陷（alias≠model 名时触发），不属本票范围，已单独报告
