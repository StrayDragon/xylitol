use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/infra-image/infra-image.feature",
    name = "受约束图片缩放"
)]
fn test_t2_image_resize() {}
#[scenario(
    path = "llmanspec/specs/infra-image/infra-image.feature",
    name = "EXIF方向边界"
)]
fn test_t2_image_exif() {}
#[scenario(
    path = "llmanspec/specs/infra-image/infra-image.feature",
    name = "超限图片格式转换"
)]
fn test_t2_image_convert() {}
#[scenario(
    path = "llmanspec/specs/infra-image/infra-image.feature",
    name = "多模态载荷边界"
)]
fn test_t2_image_multimodal() {}
#[scenario(
    path = "llmanspec/specs/infra-image/infra-image.feature",
    name = "路径至图片部件"
)]
fn test_t2_image_part() {}

#[scenario(
    path = "llmanspec/specs/infra-process/infra-process.feature",
    name = "bash路径发现"
)]
fn test_t2_proc_bash() {}
#[scenario(
    path = "llmanspec/specs/infra-process/infra-process.feature",
    name = "shell环境变量路径查找"
)]
fn test_t2_proc_shell_env() {}
#[scenario(
    path = "llmanspec/specs/infra-process/infra-process.feature",
    name = "杀死进程树回收"
)]
fn test_t2_proc_kill_tree() {}
#[scenario(
    path = "llmanspec/specs/infra-process/infra-process.feature",
    name = "子项等待带回收守卫"
)]
fn test_t2_proc_child_wait() {}
