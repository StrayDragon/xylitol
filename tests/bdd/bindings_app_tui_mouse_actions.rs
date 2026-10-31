use crate::bdd::steps_app_tui_host::{HostPumpBdd, host_pump_bdd};
use crate::bdd::steps_app_tui_interaction::{TuiInteraction, tui_interaction};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/app-tui-mouse-actions/app-tui-mouse-actions.feature",
    name = "所有常用操作均可经指针完成"
)]
async fn test_mat31_pointer_path_for_common_actions(
    tui_interaction: TuiInteraction,
    host_pump_bdd: HostPumpBdd,
) {
}

#[scenario(
    path = "llmanspec/specs/app-tui-mouse-actions/app-tui-mouse-actions.feature",
    name = "收缩后默认和弦不触发动作"
)]
fn test_mat32_contracted_defaults(tui_interaction: TuiInteraction) {}

#[scenario(
    path = "llmanspec/specs/app-tui-mouse-actions/app-tui-mouse-actions.feature",
    name = "用户重绑后动作恢复响应"
)]
async fn test_mat32_rebind_restores(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-mouse-actions/app-tui-mouse-actions.feature",
    name = "编辑区上边框右端视图 chip 可点开侧栏"
)]
async fn test_mat33_chip_opens_sidebar(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-mouse-actions/app-tui-mouse-actions.feature",
    name = "窄终端只有详情 chip"
)]
fn test_mat33_narrow_details_only(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-mouse-actions/app-tui-mouse-actions.feature",
    name = "队列条点击取回排队文本"
)]
async fn test_mat34_queue_click(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-mouse-actions/app-tui-mouse-actions.feature",
    name = "槽行首次点击只选中再次点击执行"
)]
async fn test_mat35_slot_row_click(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-mouse-actions/app-tui-mouse-actions.feature",
    name = "槽关闭 chip 回到编辑器"
)]
async fn test_mat36_close_chip(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-mouse-actions/app-tui-mouse-actions.feature",
    name = "能力盘行点击直接切换"
)]
async fn test_mat36_capabilities_row(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-mouse-actions/app-tui-mouse-actions.feature",
    name = "块头不写组合键"
)]
fn test_mat37_no_chord_names(tui_interaction: TuiInteraction) {}
