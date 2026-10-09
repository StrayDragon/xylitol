//! infra-image / infra-process 真行为步骤（c2853 从 steps_c2827 按域拆出）。
//! 步骤注册文本与拆分前一致。

use crate::bdd::helpers::with_test_timeout;
use crate::bdd::prelude::*;
use rstest_bdd_macros::{then, when};

const T2_NOISE_PNG: &[u8] = include_bytes!("../support/t2_noise_128.png");

thread_local! {
    pub(crate) static T2_IMG: RefCell<Option<(String, String)>> = const { RefCell::new(None) };
    pub(crate) static T2_PROC: RefCell<Option<(String, bool, bool)>> = const { RefCell::new(None) };
}

#[cfg(unix)]
fn t2_spawn_long_child() -> std::process::Child {
    use std::os::unix::process::CommandExt;
    std::process::Command::new("sh")
        .args(["-c", "sleep 30"])
        .process_group(0)
        .spawn()
        .expect("spawn 长进程")
}
#[cfg(windows)]
fn t2_spawn_long_child() -> std::process::Child {
    std::process::Command::new("cmd")
        .args(["/c", "ping -n 30 127.0.0.1 > nul"])
        .spawn()
        .expect("spawn 长进程")
}

fn t2_base64_ok(data: &str) -> bool {
    data.len().is_multiple_of(4)
        && data
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'/' | b'='))
}

#[when("从图片文件产出多模态载荷")]
pub(crate) fn w_image_multimodal_payload() {
    let p = std::env::temp_dir().join(format!("xylitol_t2_load_{}.png", std::process::id()));
    std::fs::write(&p, T2_NOISE_PNG).expect("写临时图");
    let r = xylitol::infra::image::agent_part_from_image_path(&p);
    let _ = std::fs::remove_file(&p);
    let part = r.expect("路径可读 MUST 产出多模态载荷");
    let (mime, data) = match part {
        xylitol::protocol::message::AgentPart::Image(ic) => {
            (ic.media_type, ic.data.unwrap_or_default())
        }
        _ => panic!("MUST 得图片构件"),
    };
    T2_IMG.with(|s| *s.borrow_mut() = Some((mime, data)));
}

#[then("base64 载荷在带宽上限内且含媒体类型")]
pub(crate) fn t_image_multimodal_payload() {
    let (mime, data) = T2_IMG.with(|s| s.borrow().clone()).expect("已产出");
    assert!(!data.is_empty(), "载荷 MUST 非空");
    assert!(data.len() <= 4_500_000, "base64 载荷 MUST 低于 4.5MB");
    assert!(mime.starts_with("image/"), "载荷 MUST 含媒体类型");
    assert!(t2_base64_ok(&data), "载荷 MUST 为合法 base64");
}

#[when("从本地图片路径装配图片构件")]
pub(crate) fn w_image_part_from_path() {
    let p = std::env::temp_dir().join(format!("xylitol_t2_part_{}.png", std::process::id()));
    std::fs::write(&p, T2_NOISE_PNG).expect("写临时图");
    let r = xylitol::infra::image::agent_part_from_image_path(&p);
    let _ = std::fs::remove_file(&p);
    let is_image = matches!(
        r.expect("路径 → 构件 MUST 成功"),
        xylitol::protocol::message::AgentPart::Image(_)
    );
    T2_IMG.with(|s| {
        *s.borrow_mut() = Some((
            if is_image {
                "image-part".into()
            } else {
                "not-image".into()
            },
            String::new(),
        ))
    });
}

#[then("得到多模态图片构件")]
pub(crate) fn t_image_part_from_path() {
    let (tag, _) = T2_IMG.with(|s| s.borrow().clone()).expect("已装配");
    assert_eq!(tag, "image-part", "MUST 得到多模态图片构件");
}

#[when("请求跨平台 bash 定位")]
pub(crate) fn w_bash_discovery() {
    let cfg = xylitol::infra::process::shell::find_bash(None);
    T2_PROC.with(|s| {
        *s.borrow_mut() = Some((
            cfg.shell.to_string_lossy().into_owned(),
            !cfg.args.is_empty(),
            false,
        ))
    });
}

#[then("返回可执行 shell 配置")]
pub(crate) fn t_bash_discovery() {
    let (shell, has_args, _) = T2_PROC.with(|s| s.borrow().clone()).expect("已定位");
    assert!(!shell.is_empty(), "MUST 返回非空 shell 路径");
    assert!(has_args, "MUST 含直执行参数");
}

#[when("以整树终止子进程")]
pub(crate) async fn w_kill_process_tree() {
    let exited = with_test_timeout(|| async {
        let mut child = t2_spawn_long_child();
        xylitol::infra::process::group::kill_process_tree(child.id());
        loop {
            if let Ok(Some(_)) = child.try_wait() {
                return true;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("kill-tree 等待 MUST 被 with_test_timeout 包裹");
    T2_PROC.with(|s| *s.borrow_mut() = Some((String::new(), false, exited)));
}

#[then("目标进程及其子进程一并结束")]
pub(crate) fn t_kill_process_tree() {
    let (_, _, exited) = T2_PROC.with(|s| s.borrow().clone()).expect("已终止");
    assert!(exited, "整树终止 MUST 回收目标进程");
}
