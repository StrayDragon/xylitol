# Design — Agent profile resume diff

## 1. 已交付

- SessionHeader 新增可选 agent_profile / agent_profile_fingerprint；formatVersion 保持 7，旧 header 缺字段按 None 解析。
- XySessionStore 新增 session_profile / set_session_profile；session 插件重写 header 持久化。
- kernel 解析 profile 时计算 SHA256 fingerprint（system_prompt + model + tools），随 assembly 传给 driver / occupancy factory。
- driver 在 new_session 写入 profile；switch_session 读取旧 profile 比较 fingerprint，不同则记录 profile_mismatch_notice；随后写入当前 profile。
- 通知文案：profile changed since session start: <old> -> <new>; resume continues without compaction。

## 2. 未交付（明确边界）

- 未接 TUI Choice 三选项（compact 后继续 / 不 compact 继续 / 取消）；当前只记录 notice，未阻断 resume。
- 未做 resume 全量重新注入；上下文仍按 build_context 正常加载。
- 未做 per-session profile binding；agent assembly 仍用 host boot profile。
- profile 不存在时仍按 bootstrap 错误处理，未回退默认 profile。
- 未写审计事件。
- 上述未交付项不得在 specs / tasks 中写成已交付。

## 3. 测试

- header 序列化 profile 字段。
- session store profile round-trip。
