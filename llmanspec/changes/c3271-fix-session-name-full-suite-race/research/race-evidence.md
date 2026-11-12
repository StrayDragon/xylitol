# 竞态证据档案（2026-11-12 深挖）

## 失败签名
- 断言：assembly_fixture_tests.rs:556 `assert_eq!(fetched.as_deref(), Some("remote name"))` → left None。
- 路径：SetSessionName Ok → 紧接 GetSessionName None（同 driver、同 store、顺序 await）。

## 复现矩阵
| 场景 | 结果 |
|---|---|
| 全量 just test / nextest --all-features（3772 测试 36 进程） | ~50%（3 败 1 过 / 历史各批 2/5） |
| 单测循环 100 次（空载） | 0 失败 |
| 单测循环 60 次（8 核 spin 满载） | 0 失败 |
| -p xylitol 子集 3 次 | 0 失败（注：过滤词为 occupancy，仅覆盖该组） |

## 已排除
- CPU 竞态本身（spin 不复现）；
- truncate_partial_tail（append.rs:748-779：仅截无换行尾，本例行完整）；
- 跨测共享状态（nextest 一测一进程；factory.rs:388 fixture 独立 tempdir+uuid）；
- fixture 双 store 实例（factory.rs:390 单一 Arc SessionManager）；
- in-process 客户端快捷路由（in_process.rs:165 是无关测试代码）。

## 嫌疑链（头号）
- sdk ports/session.rs:161-176 默认 set_session_name：SessionInfoEntry{base.id: ""}；
- log/store.rs:126 append_session_entry → append(..., with_id=false)（id 保持空）；
- log/project.rs:76-79 投影：`if let Some(id) = entry_id { map.insert }`——无 id 行丢弃。
- 未解：为何确定性丢弃呈现概率性失败（并发介入机制未定；修复选根因级消灭无 id 行，不依赖解开此谜）。

## 归属
- 先于 c3268：c3266+c3267 验收（b63532b 时点）已 2/5 失败，彼时 c3268 分支不存在；c3268 diff 不触名字路径（fixture 改动仅断言）。
