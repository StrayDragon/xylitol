---
depends_on: []
title: "Fix session name read race under full-suite load"
needs_specs_change: false
---

# 会话名读取在全量套件下的竞态（c3271）

> 2026-11-12 c3268 验收期间深挖产出。三批次验收均有记录（各 2/5 失败率）；c3268 期间 3 败 1 过（~50%），单测 160 次零失败——**先于 c3268 存在**，非其引入（c3268 diff 不触名字路径）。

## Why

`session_and_host_capabilities_match_in_process_carrier`（assembly_fixture_tests.rs:556）在全量测试套件（3772 测试 / 36 进程并发）下约 40-50% 概率失败：SetSessionName 返回 Ok("remote name") 后紧接的 GetSessionName 读回 None。单测（含 8 核 spin 满载）与 `-p xylitol` 子集均零失败——纯并发环境耦合。

## What Changes（按嫌疑排序，实施首日复核）

1. **头号嫌疑——投影层丢弃无 id 行**：`log/project.rs:76-79` 的 `if let Some(entry_id) = line.entry.entry_id() { map.insert(...) }`——无 id 的行被静默丢弃；而 XySessionStore trait 的 `append_session_entry`（log/store.rs:126）恰以 `with_id=false` 落盘 → sdk 默认 `set_session_name` 写入的 SessionInfoEntry 带 **空 id**。修复方向：`append_session_entry` 改 `with_id=true`（所有经 trait 口的条目分配 id，投影永不丢行）；或投影对无 id 行按 seq 保序收纳。
2. 备选嫌疑：manifest 缓存读不对称（W5 延迟落盘后的磁盘/内存读取路径差异）。
3. 修复后验收：全量 `just test` 连续 5 次零失败（历史失败率 ~50%，5 连绿即 97% 置信无回归）。

## 复现与已排除（证据存 research/race-evidence.md）

- 复现：仅全量套件（跨 36 二进制并发）；单测 100 次 + spin 满载 60 次 + -p xylitol 3 次均零失败。
- 已排除：CPU 竞态本身（spin 不复现）；truncate_partial_tail（仅截无换行尾）；跨测共享状态（nextest 一测一进程 + fixture 独立 tempdir+uuid）；fixture 双 store 实例（单一 Arc）。

## Capabilities

- agent-session-store（内部实现修复，无合约变更）。

## Impact

- 范围：plugins/xylitol-official-plugin-session/src/log/{store,project}.rs。

## 决策记录（2026-11-12，全部已决）

1. （已决）修复方向 = append_session_entry 统一 with_id=true（消灭无 id 行这个投影盲区）；投影侧补防御（无 id 行不再丢弃）二选一或并用，实施首日定。
2. （已决）验收 = 全量 just test 连续 5 次零失败。
3. （已决）P0 级（每批次验收都在付这个税）。
