use crate::tests::bdd::fixtures::*;
use crate::tests::bdd::helpers::*;
use crate::tests::bdd::prelude::*;
use rstest_bdd_macros::given;

#[given("有一个临时工作目录")]
fn _g_workspace(ws: &Workspace) {
    ws.init();
}

#[given("会话存储目录已初始化")]
fn _g_session_dir(sess: &XySessionStore) {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path().join("sessions");
    std::fs::create_dir_all(&d).ok();
    sess.sessions_dir.replace(Some(d.clone()));
    sess.mgr.replace(Some(SessionManager::new(d)));
}

#[given("存在文件 {path:string}")]
fn _g_empty_file(ws: &Workspace, path: String) {
    let full = ws.ws(&path);
    if let Some(p) = std::path::Path::new(&full).parent() {
        std::fs::create_dir_all(p).ok();
    }
    std::fs::write(&full, "").expect("write failed");
}

#[given("存在目录 {path:string}")]
fn _g_dir(ws: &Workspace, path: String) {
    std::fs::create_dir_all(ws.ws(&path)).ok();
}

#[given("存在空目录 {path:string}")]
fn _g_empty_dir(ws: &Workspace, path: String) {
    std::fs::create_dir_all(ws.ws(&path)).ok();
}

#[given("存在文件 {path:string} 使用CRLF行尾 内容为 {content:string}")]
fn _g_file_crlf_string(ws: &Workspace, path: String, content: String) {
    let full = ws.ws(&path);
    if let Some(p) = std::path::Path::new(&full).parent() {
        std::fs::create_dir_all(p).ok();
    }
    let normalized = strip_quotes(&content).replace('\n', "\r\n");
    std::fs::write(&full, normalized).ok();
}

#[given("存在文件 {path:string} 带UTF8_BOM 内容为 {content:string}")]
fn _g_file_bom(ws: &Workspace, path: String, content: String) {
    let full = ws.ws(&path);
    if let Some(p) = std::path::Path::new(&full).parent() {
        std::fs::create_dir_all(p).ok();
    }
    std::fs::write(&full, format!("\u{FEFF}{content}")).ok();
}

#[given("存在文件 {path:string} 包含{count:u32}行内容")]
fn _g_file_n_lines(ws: &Workspace, path: String, count: u32) {
    let full = ws.ws(&path);
    if let Some(p) = std::path::Path::new(&full).parent() {
        std::fs::create_dir_all(p).ok();
    }
    let c = (1..=count)
        .map(|i| format!("第{i}行"))
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&full, c).ok();
}

#[given("存在文件 {path:string} 包含{count:u32}行 {text:string}")]
fn _g_file_n_lines_text(ws: &Workspace, path: String, count: u32, text: String) {
    let full = ws.ws(&path);
    if let Some(p) = std::path::Path::new(&full).parent() {
        std::fs::create_dir_all(p).ok();
    }
    let c = std::iter::repeat_n(text, count as usize)
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&full, c).ok();
}

#[given("目录 {dir:string} 中存在文件 {f1:string} {f2:string} {f3:string}")]
fn _g_three_files(ws: &Workspace, dir: String, f1: String, f2: String, f3: String) {
    let d = ws.ws(&dir);
    std::fs::create_dir_all(&d).ok();
    for f in [&f1, &f2, &f3] {
        std::fs::write(format!("{d}/{f}"), "").ok();
    }
}

#[given("存在 {count:u32} 个文件匹配模式")]
fn _g_n_files_glob(ws: &Workspace, count: u32) {
    for i in 0..count {
        std::fs::write(ws.ws(&format!("file_{i}.log")), "").ok();
    }
}

#[given("目录 {dir:string} 中存在 {count:u32} 个文件")]
fn _g_n_files_in_dir(ws: &Workspace, dir: String, count: u32) {
    let d = ws.ws(&dir);
    std::fs::create_dir_all(&d).ok();
    for i in 0..count {
        std::fs::write(format!("{d}/file_{i}.txt"), "").ok();
    }
}

#[given("工作区根目录存在文件 {path:string}")]
fn _g_file_in_root(ws: &Workspace, path: String) {
    let full = ws.ws(&path);
    if let Some(p) = std::path::Path::new(&full).parent() {
        std::fs::create_dir_all(p).ok();
    }
    std::fs::write(&full, "").ok();
}

#[given("存在文件 {path:string} 内容为 {content:string}")]
fn _g_file_with_content_string(ws: &Workspace, path: String, content: String) {
    let full = ws.ws(&path);
    if let Some(p) = std::path::Path::new(&full).parent() {
        std::fs::create_dir_all(p).ok();
    }
    std::fs::write(&full, strip_quotes(&content)).expect("write failed");
}
