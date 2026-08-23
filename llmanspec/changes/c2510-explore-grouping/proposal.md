---
depends_on: []
---

# 探索分组：连续检索类工具段自动合并为一行摘要

## Why

agent 一轮里常出现连续多个低价值可见度的检索类工具调用（读文件 / grep / glob），
逐条平铺把关键决策挤出视口。现有 activity fold 提供**手动/就近**折叠
（Alt+Shift+E / Ctrl+Alt+Shift+E），但要求用户自己发现并逐个收；
缺一层「按语义自动合并」的策略，让默认视图天然安静。

## What Changes

- 在 activity_fold 的 segment 分类之上增加自动分组策略：
  连续 N 个同类检索段（读 / 搜索）合并为一个可展开摘要行
  （形如 `✱ Explored — 3 reads · 2 searches`，运行中显示进行时文案）。
- 展开交互复用既有 expandable/activity 折叠机制，不新增快捷键；
  手动展开某一段后该段退出自动分组（用户意志优先）。
- 分组阈值（最少连续数）与类别清单可配置，默认保守（≥3 才合并）。

## 非目标

- 不改 segment 的落盘结构与回放合约；
- 不合并写类 / bash / diff 等高价值段。

## Impact

- activity_fold 策略层新增分类器；scrollback 渲染行数下降；
- harness 补「连续检索合并 / 单条不合并 / 展开后保持」切片。

## Further Notes

- 现状核对与规格：[research/explore-grouping-notes.md](./research/explore-grouping-notes.md)
