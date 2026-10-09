//! infra-image / infra-process BDD bindings（c2853 按域从 bindings_c2827 拆出）。

use rstest_bdd_macros::scenario;

// infra-image r1440-1444（批 2）
#[scenario(
    path = "llmanspec/specs/infra-image/infra-image.feature",
    name = "constrained-image-resize"
)]
fn test_t2_image_resize() {}
#[scenario(
    path = "llmanspec/specs/infra-image/infra-image.feature",
    name = "exif-orientation-boundary"
)]
fn test_t2_image_exif() {}
#[scenario(
    path = "llmanspec/specs/infra-image/infra-image.feature",
    name = "overlimit-image-format-convert"
)]
fn test_t2_image_convert() {}
#[scenario(
    path = "llmanspec/specs/infra-image/infra-image.feature",
    name = "multimodal-payload-bounds"
)]
fn test_t2_image_multimodal() {}
#[scenario(
    path = "llmanspec/specs/infra-image/infra-image.feature",
    name = "path-to-image-part"
)]
fn test_t2_image_part() {}

// infra-process r1493-1496（批 2）
#[scenario(
    path = "llmanspec/specs/infra-process/infra-process.feature",
    name = "bash-path-discovery"
)]
fn test_t2_proc_bash() {}
#[scenario(
    path = "llmanspec/specs/infra-process/infra-process.feature",
    name = "shell-env-path-lookup"
)]
fn test_t2_proc_shell_env() {}
#[scenario(
    path = "llmanspec/specs/infra-process/infra-process.feature",
    name = "kill-process-tree-reaps"
)]
fn test_t2_proc_kill_tree() {}
#[scenario(
    path = "llmanspec/specs/infra-process/infra-process.feature",
    name = "child-wait-with-reap-guard"
)]
fn test_t2_proc_child_wait() {}
