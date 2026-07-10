# Design — c458-demo-theme-auto-detect

## 探测优先级（demo）

1. 显式 harness / API 注入的 scheme（最高）
2. OSC 11 背景 RGB → ITU-R BT.601 相对亮度（≥ 0.5 → Light）
3. CSI `?997;n` color-scheme 报告
4. `COLORFGBG` 环境变量（`fg;bg`，bg ≥ 7 → Light）
5. 否则 Dark

## 默认

未开 `XYLITOL_AGENT_DEMO_THEME_AUTO` 时 **MUST** 保持 Dark，即使环境有 COLORFGBG。

## 产品

MVP 固定暗色（atc3）；本变更只验证扩展点，不改产品 host。
