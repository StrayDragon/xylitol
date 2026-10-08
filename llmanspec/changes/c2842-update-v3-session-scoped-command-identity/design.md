# 设计：v3 会话命令身份透明化

## 目标

修复 served 路径上全部 session-scoped 命令的错误会话路由（`/model` writer_conflict、get_state 侧写、指令副作用落错槽）。
约束：不改 fbs schema、不重新生成、不与 JSON 轨语义分叉。

## 证据（为什么是「非 typed 化」而非补字段）

1. remote driver `unary()` **已**统一注入 `session_id`/`cwd`（`with_session`，覆盖 abort/bash 等既有调用）。JSON 轨 `params` 透传 → 路由正确。
2. v3 轨 `build_request` 把 command_backed 方法压缩进 typed `Command` 变体——fbs 各表**不含**注入字段 → 剥离。
3. 结论：v3 编码层违背自身「纯编码、语义不变」契约；修复 = 上行透传，不是给某几个表补字段（补了未来方法仍会漏）。

## 方案权衡

| 方案 | 行为 | 代价 | 判定 |
|---|---|---|---|
| A（采用）：command_backed 全 RAW 透传 | v3 上行 = 透明信封，与 JSON 轨逐字节同语义；注入字段零丢失 | 清理上行 typed Command 映射死代码（Rust 侧）；fbs/generated 保留 | ✅ 中心化、免 codegen、彻底消灭整类 |
| B：逐方法补 fbs 字段 + foryc 重生成 | typed 保留 | 需要 dev foryc 工具链（本机未装）；未来新增方法仍易漏 | 记作后续可选（工具链就绪后） |
| C：server 按连接绑定会话回落 | 服务端推断 | 打破 payload-显式设计；POST 无连接关联仍需 payload；多会话切换窗口脆弱 | 淘汰（与架构 SSOT 不符） |
| D：仅对 set_model/get_state 等点名列表走 RAW | 小面 | 名单漂移、与 typed 并存不对称，维护性差 | 淘汰 |

A 的一处语义注意：`Subscribe` / `host.describe` 的 fbs schema 已完整（Subscribe 自带 session_id），保留 typed；其余全 RAW。

## get_state 写者模型

`get_state` 是客户端状态同步的主 seam（`refresh_fixed_zone_caches`）。路由修复后它可达正确会话；让 `get_state` 在写者已装配时返回写者 `current_model`/thinking（无写者才回退 reader/Null），使所有表面（不只 c2841 已覆盖的 TUI 徽标通道）与写者状态一致。c2841 的绑定 ModelSelect 下行保留为即时通道，二者互为兜底不冲突。

## 不做

- fbs/gen 重新生成与 `Command` union 的物理删除（保留类型，仅 Rust 侧映射函数退役）；
- `internalization of obs` 等跨层议题（独立 change）。
