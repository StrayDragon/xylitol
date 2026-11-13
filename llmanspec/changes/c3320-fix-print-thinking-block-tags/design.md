# Design — Print 思考块标签配对

## 0. 复核结论（2026-11-13，HEAD f0fd974 逐条开文件核对）

5 点位：成立 4、表述需修正 1（writer 注入）。c3240/c3267 近两次触碰 print.rs 均未及思考块状态机，无已修复项。

- 「2 bool 四态」实为 **3 态可达**：(false,true) 不可达（print.rs:39-40/53-54 总同步复位）。
- MessageStart 未先闭合：成立（print.rs:52-55 直接复位，close_thinking_block :33-42 未调用）；当前 in-process ReAct 发射序不触发（MessageEnd 先于下一 MessageStart），但 print 是任意 XyEvent 流渲染器（远端 wire / 库嵌入可达）——防御性修复，优先级据此。
- 标签只在首片采样：成立（:65-66 仅块首片赋值）；后果除双闭（`</think></think>`）外还有双开（后片才现 `<think>`，:68 补开前缀并存）——不缓冲无法撤销，记为已知局限。现有单测 :209-231 只盖首片自带标签安全路径。
- close 分散：实为 **5 处**（MessageEnd/TextDelta/Error/流结束 4 调用 + MessageStart 内联隐性复位）。
- writer 注入表述修正：stdout writer 可注入（:26），但思考块标签写死 process stderr（:36-37/:68/:72-73）——「补 writer 注入」若只接现有 stdout writer 仍测不到；必须新增**可注入 err sink**。

附带发现：tests/bdd/steps_layer_invariants.rs:279-280 fixture 中 open/close 都拼成 `<think>`（close 应 `</think>`），因 stderr 不可捕获未暴露，T3 一并修。

## 1. 目标与边界

- 思考块开闭配对在任意事件序列下正确；stderr 输出格式不变（r49 钉住 `<think>` 形态）。
- 边界：不做标签风格配置（裁定见下）；已知局限——`<think>` 分片且首片未含时双开，不缓冲不撤销，记档不修。

## 2. 设计

`enum ThinkBlockPhase { Outside, Inside { model_open: bool, model_close: bool } }` 单一状态对象：

- 开：仅 ThinkingDelta arm；`model_open` = 首片含 `<think>`，不含则打补开前缀。
- 闭：单点 close，条件 `!model_close`；`model_close` 按片累积 `|= text.contains("</think>")`（修双闭）。
- MessageStart arm 先调 close 再复位 Outside（修悬挂开标签）。
- render_stream 新增可注入 err sink（第二 writer 参数或 sinks 结构），run_print 默认接 io::stderr()。

## 3. Open Question 裁定

stderr 标签风格配置：**不做**。r49 已钉形态；stderr 为人读诊断通道、仓内无解析消费方（TUI 折叠走 XyEvent）；配置后置原则。未来出现真实 stderr 消费方再立新 change。

## 4. 测试边界

单测事件序列矩阵（MessageStart 打断 / 后到闭合 / EOF 收口 / 首片自带标签，经 err sink 断言）；BDD r49 扩场景 + 修 fixture 同串 bug。
