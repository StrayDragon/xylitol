# Design：client-local 存储通道（c2470）

## 关键决策

- **落点**：`src/app/tui/storage.rs`（app 层；渲染包零感知）。路径 `<root>/<channel>/tui/<key>.json`，一 key 一文件；root 缺省 `~/.xylitol/state`，channel 缺省 `default`，env `XYLITOL_STATE_ROOT` / `XYLITOL_CHANNEL` 覆盖（dev 并行调试设 dev）。段校验 `[A-Za-z0-9][A-Za-z0-9._-]*` 防路径逃逸。
- **写串行化**：`<key>.lock` 旁路文件 + `libc::flock` 阻塞排他（先例 `infra/session/manager/write_lock.rs`；临界区仅读盘+rename，毫秒级）；锁文件持久不 unlink（r1793 教训）。落盘 = 临时文件同目录 rename（原子）；失败 best-effort 静默（最坏丢一条本地偏好）。
- **外部变更可见**：mtime 轮询 watcher（500ms 产品 / 可调），基线快照在 spawn 前调用方线程取（spawn 竞态不丢首写）；变更 key 经 `std::sync::mpsc` 通道交会话，主循环 tick 轮询合并——不引 notify 依赖、不加 select 臂。
- **seed×channel 合成**：任何 seed 路径（启动/恢复/切换）替换历史后统一 `merge_client_history()`（单钩子 `seed_editor_history_from_texts`），通道条目去重置顶——切换不丢跨实例历史。
- **发送落盘**：input_policy 四个提交路径在 root 释放后 `persist_client_history`（不占 root 借用）；失败静默。
- **迁移范围**：本期只迁 editor ↑/↓ 历史（首个条目，验证全链路）；keybindings/fold 等后续按同机制增量接入（通道 API 已泛化 `load/update/watch`）。

## 权衡

- 轮询 vs notify：免重依赖、测试确定性强；500ms 延迟对偏好类态无感。
- 进程内历史与通道合并策略：通道条目只增去重，不反向清空；cap 对齐包内 editor（100）。
- 不做跨设备同步 / 会话数据（proposal 非目标）；`packages/xylitol-tui` 仅加 `merge_history` / `history_texts` 最小面。
