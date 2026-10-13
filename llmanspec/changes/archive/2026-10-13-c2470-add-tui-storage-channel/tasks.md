# Tasks

## T1 存储通道核心

- [x] `src/app/tui/storage.rs`：`ClientStorage`（分桶/段校验/load/flock+原子 update/mtime watcher/编辑器历史便捷层）+ 单测
- [x] `app/tui/mod.rs`：`pub mod storage`

## T2 编辑器历史接线

- [x] 包 editor `merge_history`/`history_texts` + root 透传
- [x] HostSession：`set_client_storage` / `set_client_history_receiver` / `persist_client_history`（input_policy 四提交路径）/ `merge_client_history`（seed 钩子）/ `drain_client_history_updates`（tick 轮询）
- [x] `TuiRunOptions.client_storage` + cli 生产装配（`from_env`）+ watcher 接线

## T3 specs 与 BDD

- [x] `app-tui-storage` capability：r2898 分桶段校验 / r2899 串行化原子 / r2900 外部可见 / r2901 双实例互见 / r2902 编辑器历史
- [x] `tests/bdd/steps_storage.rs` + `bindings_storage.rs` + suite 注册；编辑器历史场景走 host pump 真实按键路径

## T4 验证

- [x] `cargo test --all-features --lib`（含 storage 单测）与 `--test bdd` 全绿；clippy 零警告
- [x] `llman-sdd validate c2470-add-tui-storage-channel --strict` 绿
