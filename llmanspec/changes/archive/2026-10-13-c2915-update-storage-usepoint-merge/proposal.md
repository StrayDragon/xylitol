---
depends_on: []
title: 存储通道外部感知降级为使用点懒并盘（撤 watcher 常驻线程）
needs_specs_change: true
branch: sdd/c2915-update-storage-usepoint-merge
base_branch: main
base_sha: a9f35b1b5dc9d5c9122afa3dc5bc5c7d37a391ce
---

# 存储通道外部感知降级为使用点懒并盘（撤 watcher 常驻线程）

## Why

c2470 的 mtime 轮询 watcher 是「外部变更可见」的推送实现：常驻线程周期扫目录、mpsc 投递、主循环 tick 合并。复盘（2026-10-13 review）：多实例同会话并用的场景本就少见，推送语义对偏好态收益趋零，而成本是常驻线程 + 周期扫盘 + 自写回环。按「低占用、场景少则不做」的产品取向，降级为**使用点懒并盘**：↑ 召回前重读通道并去重合并——外部实例的写入在下一次 ↑ 即可见，无常驻线程。

## What Changes

- 撤 `ClientStorage::spawn_watcher`（含 `snapshot_keys` / `mtime_millis` 辅助与 watcher 单测）、`HostSession::set_client_history_receiver` / `drain_client_history_updates` / `client_history_rx` 字段、`run_host_loop` 装配与 tick 轮询。
- 增使用点：host 输入入口对**无修饰 ↑** 先 `merge_client_history()`（无通道 no-op），↑ 召回即见同通道他实例写入；seed 路径合并不变。
- specs：`app-tui-storage` 撤 r2900（推送式「外部变更可见」）；r2902 措辞改「↑ 召回（使用点）须先并盘」，场景第二段随动（轮询 → ↑ 召回）。r2898 / r2899 / r2901 不动。
- BDD：watcher 三步与绑定撤除；c2470 双实例场景改「他实例另写后 本端按 ↑ 召回」。

## 非目标

- 不撤通道本身（分桶 / flock / 原子写 / 编辑器历史持久化保留，r2898 / r2899 照旧）。
- 不动 seed 合并语义（r2902 前半不变）。
