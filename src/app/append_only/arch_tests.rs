//! Architecture probe (atao7): import boundary + XyDriver reuse markers.

use std::path::PathBuf;

fn append_only_sources() -> Vec<PathBuf> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/app/append_only");
    let mut files = Vec::new();
    for entry in std::fs::read_dir(&root).expect("append_only dir") {
        let entry = entry.expect("entry");
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            files.push(path);
        }
    }
    files
}

#[test]
fn atao7_no_infra_or_capabilities_reach_in() {
    for path in append_only_sources() {
        let src = std::fs::read_to_string(&path).expect("read");
        // Allow listing forbidden patterns; skip this file's string literals in comments.
        if path.file_name().and_then(|n| n.to_str()) == Some("arch_tests.rs") {
            continue;
        }
        assert!(
            !src.contains("crate::infra::"),
            "{} must not reach-in crate::infra::",
            path.display()
        );
        assert!(
            !src.contains("crate::agent::capabilities"),
            "{} must not reach-in agent::capabilities",
            path.display()
        );
        assert!(
            !src.contains("crate::agent::runtime"),
            "{} must not reach-in agent::runtime",
            path.display()
        );
    }
}

#[test]
fn atao7_reuses_xydriver_seam_in_host() {
    let host = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/app/append_only/host.rs");
    let src = std::fs::read_to_string(host).expect("host");
    assert!(src.contains("XyDriver"));
    assert!(src.contains("driver.run("));
    assert!(src.contains("driver.abort("));
    assert!(src.contains("InteractionMode::Inline"));
    // Deliberately not copying mainline fold chrome
    assert!(!src.contains("activity_fold"));
    assert!(!src.contains("install_fold_triangle"));
    assert!(!src.contains("Command Plate") && !src.contains("command_plate"));
}

#[test]
fn atao3_no_expandable_blocks_api() {
    use crate::app::append_only::{AppendOnlyModel, BlockKind};
    let tmp = tempfile::tempdir().unwrap();
    let mut model = AppendOnlyModel::new(Some(tmp.path().to_path_buf()));
    model
        .commit_capped(BlockKind::ToolBashAssistant, "t", "1\n2\n3\n4\n", "t")
        .unwrap();
    assert!(!model.any_expandable());
}
