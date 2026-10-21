use crate::bdd::steps_app_tui_host::{HostPumpBdd, host_pump_bdd};
use crate::bdd::steps_storage::StorageBdd;
use crate::bdd::steps_storage::storage_bdd;
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/app-tui-storage/app-tui-storage.feature",
    name = "存储通道分区与段"
)]
fn test_cs_partition(storage_bdd: StorageBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-storage/app-tui-storage.feature",
    name = "并发写入全部落地原子地"
)]
fn test_cs_concurrent(storage_bdd: StorageBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-storage/app-tui-storage.feature",
    name = "损坏文件回落返回"
)]
fn test_cs_corrupt(storage_bdd: StorageBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-storage/app-tui-storage.feature",
    name = "双实例相互可见性"
)]
fn test_cs_dual(storage_bdd: StorageBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-storage/app-tui-storage.feature",
    name = "编辑器历史持久化与合并"
)]
fn test_cs_editor_history(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-storage/app-tui-storage.feature",
    name = "编辑器历史合并召回顺序"
)]
fn test_cs_editor_history_recall_order(host_pump_bdd: HostPumpBdd) {}

#[allow(unused_imports)]
use host_pump_bdd as _;
