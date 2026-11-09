use crate::bdd::steps_app_tui_host::{HostPumpBdd, host_pump_bdd};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/infra-clipboard/infra-clipboard.feature",
    name = "拖选选择复制助手正文"
)]
fn test_cb_r13_drag_copy() {}

#[scenario(
    path = "llmanspec/specs/infra-clipboard/infra-clipboard.feature",
    name = "缺省模式行内"
)]
fn test_cb_r14_default_mode() {}

#[scenario(
    path = "llmanspec/specs/infra-clipboard/infra-clipboard.feature",
    name = "复制提示可观察"
)]
fn test_cb_r15_copy_notice() {}

#[scenario(
    path = "llmanspec/specs/infra-clipboard/infra-clipboard.feature",
    name = "OSC52远端回退"
)]
fn test_cb_r16_osc52() {}

#[scenario(
    path = "llmanspec/specs/infra-clipboard/infra-clipboard.feature",
    name = "空选区静默"
)]
fn test_cb_r17_empty_selection() {}

#[scenario(
    path = "llmanspec/specs/infra-clipboard/infra-clipboard.feature",
    name = "编辑器复制路径仅"
)]
fn test_cb_r18_editor_path() {}

#[scenario(
    path = "llmanspec/specs/infra-clipboard/infra-clipboard.feature",
    name = "图片粘贴来自Host"
)]
async fn test_cb_r19_image_paste(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/infra-clipboard/infra-clipboard.feature",
    name = "文本读取经Host泵"
)]
async fn test_cb_r20_text_read(host_pump_bdd: HostPumpBdd) {}
