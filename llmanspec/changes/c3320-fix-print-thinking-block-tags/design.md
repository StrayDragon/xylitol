# Design — Print 思考块标签配对

## 1. 目标与边界

- 思考块开闭配对在任意事件序列下正确；输出格式不变。
- 边界：不改 stderr 标签风格（<think> 形态保留）。

## 2. 设计

ThinkBlockPhase { Outside, Inside { has_tags: bool } } 单一状态对象；MessageStart 前先收口旧块（发闭合标签）；标签检测在每片执行（首片未检出后片检出时补开标签）；close 单点。render_stream 的闭合断言测试补 writer 注入。

## 3. 测试边界

单测：事件序列矩阵（ThinkingDelta 起止 / MessageStart 打断 / EOF 收口）；cli-print BDD 回归。
