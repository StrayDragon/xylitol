use crate::bdd::steps_cli_surface::{AttachBdd, attach_bdd};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/user-experience/user-experience.feature",
    name = "登录引导引用文档"
)]
fn test_ux_login() {}

#[scenario(
    path = "llmanspec/specs/user-experience/user-experience.feature",
    name = "无模型引导"
)]
fn test_ux_no_models() {}

#[scenario(
    path = "llmanspec/specs/user-experience/user-experience.feature",
    name = "未设模型显示"
)]
fn test_ux_unset() {}

#[scenario(
    path = "llmanspec/specs/user-experience/user-experience.feature",
    name = "无API键名称厂商"
)]
fn test_ux_no_key() {}

#[scenario(
    path = "llmanspec/specs/user-experience/user-experience.feature",
    name = "确保失败诊断可操作"
)]
fn test_ux_ensure_diag(attach_bdd: AttachBdd) {}

#[scenario(
    path = "llmanspec/specs/user-experience/user-experience.feature",
    name = "替换失败诊断可操作"
)]
fn test_ux_replace_timeout(attach_bdd: AttachBdd) {}

#[scenario(
    path = "llmanspec/specs/user-experience/user-experience.feature",
    name = "客户端过旧诊断可操作"
)]
fn test_ux_client_too_old(attach_bdd: AttachBdd) {}

#[scenario(
    path = "llmanspec/specs/user-experience/user-experience.feature",
    name = "Host过旧处置可操作"
)]
fn test_ux_host_too_old(attach_bdd: AttachBdd) {}
