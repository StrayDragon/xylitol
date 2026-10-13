# client-local 存储通道 外部对照笔记（c2470）

> 调研来源：外部参考实现（生产级 coding agent），2026-08-23 摘录。
> 主文件：其 TUI 包 `src/context/storage.tsx`（已逐行核对前 80 行）。
> 本笔记仅作选型对照；关键结论已摘要进 proposal。

## 参考实现一手证据

**机制**（storage.tsx:47-127）：

- 存储 = `<state-root>/<channel>/tui/<key>.json`，一个 key 一个 JSON 文件；
- 读：启动时同步读盘，失败回退 initial；
- 写：`Flock.withLock(file)` 内完成 读盘 → mutate draft → `writeJsonAtomic`
  （临时文件+rename）→ reconcile 回内存 store——多进程写安全 + 原子落盘；
- 外部变更感知：`fs.watch(directory)` 监听原子写的 rename 事件风暴，50ms 防抖后逐 entry 重载，
  `reconcile(next, { key })` 按稳定 key 保数组对象身份（避免行重建破坏动画）；
- `memory(key)`：进程内 memoized store，跨热重载存活、退出即失，值不必可序列化。

**channel 分桶动机**（其根 AGENTS.md:12 + 版本模块）：

- channel 来自构建期环境变量（dev / beta / latest…），存储按 channel 分桶；
- 开发工作流 `dev:live` 设 dev channel：开发版前端连已安装版 server 时，
  与正式客户端共享同一份 client-local state（tabs 等一致）；同时不同通道互不污染。
- segment 校验：channel/key 必须匹配 `[a-zA-Z0-9][a-zA-Z0-9._-]*`，防路径逃逸。

**一致性来源**：不是网络同步，而是「磁盘为真值 + 文件锁 + watch 重载」。
任何同 channel 实例写文件，其他实例 watch 到即整体 reconcile。
已知竞态有显式处理：session-tabs.tsx:80-86 用 cancelledTabs 集合防「在途写入期间关闭的 tab
被迟到的注册复活」。

## 设计动机

- client-local 态的生命周期与 server 无关（tabs、布局、frecency……），
  用 IPC / 数据库都过重；磁盘共享是零基建的多实例一致性方案；
- flock 把并发正确性收敛到写路径一处，读侧永远可以随便读（最坏读到旧值，
  watch 会追上）。

## xylitol 现状核对（2026-08-23）

- editor ↑/↓ 历史 seed：`src/app/tui/host/editor_history.rs`（c1560/ath12）独立机制；
- activity fold 设置：`set_activity_fold_settings` 独立持久化点；
- keybindings.json / theme 热重载（c1090/c1095）各走各的 reload 路径；
- 结论：散点多、模式不一、无双实例一致性保障；新增一类状态要重造读写轮子。

## 落点判断

- Rust 侧实现极轻：flock（fs2/fd-lock 类 crate）+ 临时文件 rename + notify 防抖 +
  serde JSON。channel 分桶直接映射本仓 dev/正式双形态并行调试需求。
- 边界：只承载 client-local 态（偏好/布局/历史），会话数据仍归 Host；
  渲染包 `xylitol-tui` 不感知存储细节（app 层接线）。
