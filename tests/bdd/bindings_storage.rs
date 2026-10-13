//! app-tui-storage BDD 绑定（c2470）。

use crate::bdd::steps_app_tui_host::{HostPumpBdd, host_pump_bdd};
use crate::bdd::steps_storage::StorageBdd;
use crate::bdd::steps_storage::storage_bdd;
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/app-tui-storage/app-tui-storage.feature",
    name = "storage-channel-partition-and-segments"
)]
fn test_cs_partition(storage_bdd: StorageBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-storage/app-tui-storage.feature",
    name = "concurrent-writes-all-land-atomically"
)]
fn test_cs_concurrent(storage_bdd: StorageBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-storage/app-tui-storage.feature",
    name = "corrupt-file-falls-back"
)]
fn test_cs_corrupt(storage_bdd: StorageBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-storage/app-tui-storage.feature",
    name = "cross-instance-write-is-observed"
)]
fn test_cs_watch(storage_bdd: StorageBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-storage/app-tui-storage.feature",
    name = "dual-instance-mutual-visibility"
)]
fn test_cs_dual(storage_bdd: StorageBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-storage/app-tui-storage.feature",
    name = "editor-history-persist-and-merge"
)]
fn test_cs_editor_history(host_pump_bdd: HostPumpBdd) {}

// 保留 host_pump_bdd 导入使用（编辑器历史场景经其 fixture 装配）。
#[allow(unused_imports)]
use host_pump_bdd as _;
