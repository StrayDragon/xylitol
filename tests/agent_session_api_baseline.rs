#![cfg(test)]

use std::fs;
use std::path::PathBuf;

use insta::assert_snapshot;

fn session_sources() -> Vec<PathBuf> {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.push("crates/xylitol-runtime/src/agent/capabilities");
    assert!(
        dir.is_dir(),
        "cannot locate capabilities module at {}",
        dir.display()
    );
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("read capabilities dir")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "rs"))
        .collect();
    files.sort();
    files
}

fn extract_public_items(src: &str) -> Vec<String> {
    let mut items: Vec<String> = Vec::new();
    let lines: Vec<&str> = src.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let trimmed = lines[i].trim_start();
        let is_public = trimmed.starts_with("pub ")
            || trimmed.starts_with("pub(async ")
            || trimmed.starts_with("pub async ")
            || trimmed.starts_with("pub use ")
            || trimmed.starts_with("pub struct ")
            || trimmed.starts_with("pub enum ")
            || trimmed.starts_with("pub type ");
        let is_restricted = trimmed.starts_with("pub(");
        if is_public && !is_restricted {
            let mut acc = String::new();
            let mut depth: i32 = 0;
            let mut terminated = false;
            while i < lines.len() {
                let l = lines[i];
                acc.push_str(l.trim());
                acc.push(' ');
                for ch in l.chars() {
                    match ch {
                        '(' | '[' => depth += 1,
                        ')' | ']' => depth -= 1,
                        '{' if depth == 0 => {
                            terminated = true;
                        }
                        ';' if depth == 0 => {
                            terminated = true;
                        }
                        _ => {}
                    }
                    if terminated {
                        break;
                    }
                }
                if terminated {
                    break;
                }
                i += 1;
            }
            push_normalized(&acc, &mut items);
        }
        i += 1;
    }
    items
}

fn push_normalized(acc: &str, items: &mut Vec<String>) {
    let collapsed: String = acc.split_whitespace().collect::<Vec<_>>().join(" ");
    let cleaned = collapsed
        .trim_end_matches('{')
        .trim_end_matches(';')
        .trim_end()
        .to_string();
    if !cleaned.is_empty() {
        items.push(cleaned);
    }
}

#[test]
fn agent_session_public_api_surface_baseline() {
    let mut all_items: Vec<String> = Vec::new();
    for file in session_sources() {
        let src =
            fs::read_to_string(&file).unwrap_or_else(|e| panic!("read {}: {}", file.display(), e));
        all_items.extend(extract_public_items(&src));
    }
    all_items.sort();
    all_items.dedup();
    assert_snapshot!(all_items.join("\n"));
}
