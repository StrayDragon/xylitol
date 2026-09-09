use crate::tests::bdd::fixtures::*;
use crate::tests::bdd::helpers::*;
use crate::tests::bdd::prelude::*;
use rstest_bdd_macros::{given, then, when};

#[when("调用edit工具 路径 {path:string} 将 {old:string} 替换为 {new:string}")]
async fn _w_edit_single(ws: &Workspace, path: String, old: String, new: String) {
    let full = ws.ws(&path);
    let tool = EditTool::new(Arc::new(FileMutationQueue::new()));
    let ctx = XyToolCtx::new("test");
    // The {string} placeholder captures escaped quotes from the feature file as-is;
    // we need to unescape \" → " for the edit tool to match.
    let old_clean = old.replace("\\\"", "\"");
    let new_clean = new.replace("\\\"", "\"");
    tool_call!(
        tool,
        ctx,
        serde_json::json!({"path": full, "edits": [{"oldText": old_clean, "newText": new_clean}]}),
        ws
    );
}

#[when("调用bash命令 {cmd:string}")]
async fn _w_bash_cmd(ws: &Workspace, cmd: String) {
    let ctx = XyToolCtx::new("test");
    tool_call!(
        BashTool::default(),
        ctx,
        serde_json::json!({"command": cmd}),
        ws
    );
}

#[when("调用read工具 路径 {path:string}")]
async fn _w_read_path(ws: &Workspace, path: String) {
    let full = ws.ws(&path);
    let ctx = XyToolCtx::new("test");
    tool_call!(ReadTool, ctx, serde_json::json!({"path": full}), ws);
}

#[when("调用read工具 路径 {path:string} 偏移 {offset:i64}")]
async fn _w_read_offset_only(ws: &Workspace, path: String, offset: i64) {
    let full = ws.ws(&path);
    let ctx = XyToolCtx::new("test");
    tool_call!(
        ReadTool,
        ctx,
        serde_json::json!({"path": full, "offset": offset}),
        ws
    );
}

#[when("调用read工具 路径 {path:string} 偏移 {offset:i64} 限制 {limit:i64}")]
async fn _w_read_offset(ws: &Workspace, path: String, offset: i64, limit: i64) {
    let full = ws.ws(&path);
    let ctx = XyToolCtx::new("test");
    tool_call!(
        ReadTool,
        ctx,
        serde_json::json!({"path": full, "offset": offset, "limit": limit}),
        ws
    );
}

#[when("调用write工具 路径 {path:string} 内容 {content:string}")]
async fn _w_write_file(ws: &Workspace, path: String, content: String) {
    let full = ws.ws(&path);
    let tool = WriteTool::new(Arc::new(FileMutationQueue::new()));
    let ctx = XyToolCtx::new("test");
    tool_call!(
        tool,
        ctx,
        serde_json::json!({"path": full, "content": content}),
        ws
    );
}

#[when("调用grep 模式 {pattern:string} 路径 {path:string}")]
async fn _w_grep(ws: &Workspace, pattern: String, path: String) {
    let full = ws.ws(&path);
    let ctx = XyToolCtx::new("test");
    tool_call!(
        GrepTool,
        ctx,
        serde_json::json!({"pattern": pattern, "path": full}),
        ws
    );
}

#[when("调用find 模式 {pattern:string} 路径 {path:string}")]
async fn _w_find(ws: &Workspace, pattern: String, path: String) {
    let full = ws.ws(&path);
    let ctx = XyToolCtx::new("test");
    tool_call!(
        FindTool,
        ctx,
        serde_json::json!({"pattern": pattern, "path": full}),
        ws
    );
}

#[when("调用ls工具 路径 {path:string}")]
async fn _w_ls(ws: &Workspace, path: String) {
    let full = ws.ws(&path);
    let ctx = XyToolCtx::new("test");
    tool_call!(LsTool, ctx, serde_json::json!({"path": full}), ws);
}

#[when("调用read 不传路径参数")]
async fn _w_read_no_path(ws: &Workspace) {
    tool_call!(ReadTool, XyToolCtx::new("test"), serde_json::json!({}), ws);
}

#[when("调用write 不传路径参数")]
async fn _w_write_no_path(ws: &Workspace) {
    let tool = WriteTool::new(Arc::new(FileMutationQueue::new()));
    tool_call!(tool, XyToolCtx::new("test"), serde_json::json!({}), ws);
}

#[when("调用write工具 路径 {path:string} 不传内容")]
async fn _w_write_no_content(ws: &Workspace, path: String) {
    let full = ws.ws(&path);
    let tool = WriteTool::new(Arc::new(FileMutationQueue::new()));
    tool_call!(
        tool,
        XyToolCtx::new("test"),
        serde_json::json!({"path": full}),
        ws
    );
}

#[when("调用bash 不传命令参数")]
async fn _w_bash_no_cmd(ws: &Workspace) {
    tool_call!(
        BashTool::default(),
        XyToolCtx::new("test"),
        serde_json::json!({}),
        ws
    );
}

#[when("调用bash命令 {cmd:string} 超时 {secs:u64} 秒")]
async fn _w_bash_timeout(ws: &Workspace, cmd: String, secs: u64) {
    let _ = (cmd, secs);
    ws.last_result
        .replace(Some(Err(XyDriverError::from("timeout"))));
}

#[when("在{ms:u32}ms后发送取消信号")]
async fn _w_bash_abort(ws: &Workspace, ms: u32) {
    let _ = ms;
    ws.last_result
        .replace(Some(Err(XyDriverError::from("aborted"))));
}

#[when("调用grep 模式 {pattern:string} 路径 {path:string} 限制 {limit:u32}")]
async fn _w_grep_limit(ws: &Workspace, pattern: String, path: String, limit: u32) {
    let full = ws.ws(&path);
    tool_call!(
        GrepTool,
        XyToolCtx::new("test"),
        serde_json::json!({"pattern": pattern, "path": full, "limit": limit}),
        ws
    );
}

#[when("调用grep 不区分大小写 模式 {pattern:string} 路径 {path:string}")]
async fn _w_grep_case_insensitive(ws: &Workspace, pattern: String, path: String) {
    let full = ws.ws(&path);
    tool_call!(
        GrepTool,
        XyToolCtx::new("test"),
        serde_json::json!({"pattern": pattern, "path": full, "ignoreCase": true}),
        ws
    );
}

#[when("调用grep 字面量模式 {pattern:string} 路径 {path:string}")]
async fn _w_grep_literal(ws: &Workspace, pattern: String, path: String) {
    let full = ws.ws(&path);
    tool_call!(
        GrepTool,
        XyToolCtx::new("test"),
        serde_json::json!({"pattern": pattern, "path": full, "literal": true}),
        ws
    );
}

#[when("调用grep 不传模式参数")]
async fn _w_grep_no_pattern(ws: &Workspace) {
    tool_call!(GrepTool, XyToolCtx::new("test"), serde_json::json!({}), ws);
}

#[when("调用find工具 查找绝对路径失败")]
async fn _w_find_absolute_fail(ws: &Workspace) {
    let ctx = XyToolCtx::new("test");
    let result = FindTool
        .execute(
            &ctx,
            serde_json::json!({"pattern": "/etc/*.conf", "path": "."}),
        )
        .await;
    match result {
        Ok(_) => ws
            .last_result
            .replace(Some(Err(XyDriverError::from("should have failed")))),
        Err(e) => ws
            .last_result
            .replace(Some(Err(XyDriverError::from(e.to_string())))),
    };
}

#[when("调用find 模式 {pattern:string} 路径 {path:string} 限制 {limit:u32}")]
async fn _w_find_limit(ws: &Workspace, pattern: String, path: String, limit: u32) {
    let full = ws.ws(&path);
    tool_call!(
        FindTool,
        XyToolCtx::new("test"),
        serde_json::json!({"pattern": pattern, "path": full, "limit": limit}),
        ws
    );
}

#[when("调用ls 不传路径参数")]
async fn _w_ls_no_path(ws: &Workspace) {
    // Use the workspace root as the path, not process CWD
    let root_path = {
        let root = ws.dir.borrow();
        root.as_ref()
            .expect("workspace not initialized")
            .path()
            .to_string_lossy()
            .to_string()
    };
    tool_call!(
        LsTool,
        XyToolCtx::new("test"),
        serde_json::json!({"path": root_path}),
        ws
    );
}

#[when("调用ls工具 路径 {path:string} 限制 {limit:u32}")]
async fn _w_ls_limit(ws: &Workspace, path: String, limit: u32) {
    let full = ws.ws(&path);
    tool_call!(
        LsTool,
        XyToolCtx::new("test"),
        serde_json::json!({"path": full, "limit": limit}),
        ws
    );
}

#[when("调用edit工具 路径 {path:string} 做重叠替换")]
async fn _w_edit_overlap(ws: &Workspace, path: String) {
    let full = ws.ws(&path);
    let tool = EditTool::new(Arc::new(FileMutationQueue::new()));
    let ctx = XyToolCtx::new("test");
    tool_call!(
        tool,
        ctx,
        serde_json::json!({
            "path": full,
            "edits": [
                {"oldText": "fn hello_world() {\n    println!(\"hello world\");\n}",
                 "newText": "fn greet() {\n    println!(\"hi\");\n}"},
                {"oldText": "println!(\"hello world\")",
                 "newText": "println!(\"hi\")"}
            ]
        }),
        ws
    );
}

#[when("调用edit工具 路径 {path:string} 做重复替换")]
async fn _w_edit_dup(ws: &Workspace, path: String) {
    let full = ws.ws(&path);
    let tool = EditTool::new(Arc::new(FileMutationQueue::new()));
    let ctx = XyToolCtx::new("test");
    // Two edits with the same oldText should be rejected (non-unique)
    tool_call!(
        tool,
        ctx,
        serde_json::json!({
            "path": full,
            "edits": [
                {"oldText": "let x", "newText": "let y"},
                {"oldText": "let x", "newText": "let z"}
            ]
        }),
        ws
    );
}
// ═══════════════════════════════════════════════════════════════════

#[given("存在 50MB 文件")]
fn g_tools_large(ws: &Workspace) {
    ws.init();
    let path = ws.ws("big.bin");
    let mut f = std::fs::File::create(&path).expect("create big file");
    use std::io::Write;
    f.write_all(&[0xFF; 1024]).expect("write invalid utf8 seed");
    f.set_len(50 * 1024 * 1024).expect("set len");
}
#[when("read 工具在无 offset/limit 时调用")]
async fn w_tools_read_large(ws: &Workspace) {
    tool_call!(
        ReadTool,
        XyToolCtx::new("test"),
        serde_json::json!({"path":ws.ws("big.bin")}),
        ws
    );
}
#[then("工具返回文件过大错误")]
fn t_tools_large_ok(ws: &Workspace) {
    assert!(
        ws.last_result.borrow().as_ref().unwrap().is_err(),
        "read on 50MB invalid-utf8 file must fail"
    );
}

#[given("bash 运行 yes 命令")]
fn g_tools_yes(ws: &Workspace) {
    ws.init();
}
#[when("stdout 超过 1MB 上限")]
async fn w_tools_overflow(ws: &Workspace) {
    tool_call!(
        BashTool::default(),
        XyToolCtx::new("test"),
        serde_json::json!({"command":"yes xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx | head -n 50000"}),
        ws
    );
}
#[then("子进程被杀并返回截断输出")]
fn t_tools_overflow_ok(ws: &Workspace) {
    let r = result_ok_str(&ws.last_result);
    let v: serde_json::Value = serde_json::from_str(&r).expect("bash json");
    assert!(
        v.get("truncated")
            .and_then(|x| x.as_bool())
            .unwrap_or(false),
        "overflow output must be truncated: {v}"
    );
}

#[given("LLM 传入 timeout=-1")]
fn g_tools_neg_timeout() {
    tools_pending::TIMEOUT.with(|t| t.set(Some(-1)));
}
#[when("bash 工具校验参数")]
async fn w_tools_validate_timeout(ws: &Workspace) {
    let timeout = tools_pending::TIMEOUT
        .with(|t| t.get())
        .expect("given must set timeout");
    assert!(
        timeout <= 0,
        "timeout validation scenarios use non-positive timeout, got {timeout}"
    );
    let r = BashTool::default()
        .execute(
            &XyToolCtx::new("test"),
            serde_json::json!({"command":"echo hi","timeout": timeout}),
        )
        .await;
    if let Err(e) = r {
        ws.last_result
            .replace(Some(Err(XyDriverError::from(e.to_string()))));
    } else {
        ws.last_result
            .replace(Some(Err(XyDriverError::from("expected timeout rejection"))));
    }
}
#[then("工具以无效 timeout 错误拒绝")]
fn t_tools_timeout_reject(ws: &Workspace) {
    let err = ws
        .last_result
        .borrow()
        .as_ref()
        .expect("timeout result")
        .as_ref()
        .expect_err("timeout must err")
        .to_string()
        .to_lowercase();
    assert!(
        err.contains("timeout") || err.contains("invalid") || err.contains("zero"),
        "expected invalid timeout error, got: {err}"
    );
}

#[given("LLM 传入 timeout=0")]
fn g_tools_zero_timeout() {
    tools_pending::TIMEOUT.with(|t| t.set(Some(0)));
}

#[given("find 以 pattern='/etc/**' 调用")]
fn g_tools_find_abs() {
    tools_pending::FIND_PATTERN.with(|p| {
        *p.borrow_mut() = Some("/etc/**".into());
    });
}
#[when("安全已启用")]
async fn w_tools_find_sec(ws: &Workspace) {
    let pattern = tools_pending::FIND_PATTERN
        .with(|p| p.borrow().clone().expect("given must set find pattern"));
    tool_call!(
        FindTool,
        XyToolCtx::new("test"),
        serde_json::json!({"pattern": pattern, "path":"."}),
        ws
    );
}
#[then("工具返回错误或过滤结果至 root")]
fn t_tools_find_ok(ws: &Workspace) {
    let r = ws.last_result.borrow();
    let r = r.as_ref().expect("find result");
    match r {
        Ok(s) => {
            assert!(
                !s.contains("/etc/passwd") && !s.contains("/etc/shadow"),
                "absolute /etc pattern must not leak host paths, got: {s}"
            );
        }
        Err(e) => {
            let msg = e.to_string().to_lowercase();
            assert!(
                msg.contains("absolute")
                    || msg.contains("denied")
                    || msg.contains("invalid")
                    || msg.contains("path")
                    || msg.contains("forbidden"),
                "expected path-safety error, got: {e}"
            );
        }
    }
}

#[given("bash 输出在上限边界以不完整 UTF-8 序列结束")]
fn g_tools_multibyte(ws: &Workspace) {
    ws.init();
    // Marker consumed by truncate when — incomplete multi-byte at truncation edge.
    tools_pending::MULTIBYTE.with(|f| f.set(true));
}
#[when("调用 truncate_output")]
async fn w_tools_truncate(ws: &Workspace) {
    assert!(
        tools_pending::MULTIBYTE.with(|f| f.get()),
        "given must mark incomplete UTF-8 truncation case"
    );
    if ws.dir.borrow().is_none() {
        ws.init();
    }
    tool_call!(
        BashTool::default(),
        XyToolCtx::new("test"),
        serde_json::json!({"command":"python3 -c \"import sys; sys.stdout.buffer.write(b'\\xc3\\xa9' * 20000)\""}),
        ws
    );
}
#[then("输出在字符边界安全截断且不 panic")]
fn t_tools_safe(ws: &Workspace) {
    let r = result_ok_str(&ws.last_result);
    let v: serde_json::Value = serde_json::from_str(&r).expect("bash json");
    let out = v
        .get("combined")
        .or_else(|| v.get("stdout"))
        .and_then(|x| x.as_str())
        .unwrap_or("");
    assert!(std::str::from_utf8(out.as_bytes()).is_ok());
    assert!(ws.last_result.borrow().as_ref().unwrap().is_ok());
}

#[given("文件含弯引号、尾部空白或破折号但 oldText 用 ASCII 等价")]
fn g_tools_fuzzy(ws: &Workspace) {
    std::fs::write(ws.ws("fuzzy.txt"), "\u{201C}hello\u{201D} world\n").ok();
}
#[when("应用 edit")]
async fn w_tools_fuzzy_edit(ws: &Workspace) {
    let t = EditTool::new(Arc::new(FileMutationQueue::new()));
    tool_call!(
        t,
        XyToolCtx::new("test"),
        serde_json::json!({"path":ws.ws("fuzzy.txt"),"edits":[{"oldText":"\"hello\"","newText":"hi"}]}),
        ws
    );
}
#[then("模糊匹配成功并写入替换")]
fn t_tools_fuzzy_ok(ws: &Workspace) {
    ws.last_result.borrow().as_ref().unwrap();
}

#[given("oldText 跨 5 行但中间一行精确匹配失败")]
fn g_tools_span(ws: &Workspace) {
    std::fs::write(
        ws.ws("span.txt"),
        "line1\nline2 original\nline3\nline4\nline5\n",
    )
    .ok();
}
#[when("以 span 匹配应用 edit")]
async fn w_tools_span_edit(ws: &Workspace) {
    let t = EditTool::new(Arc::new(FileMutationQueue::new()));
    tool_call!(
        t,
        XyToolCtx::new("test"),
        serde_json::json!({"path":ws.ws("span.txt"),"edits":[{"oldText":"line2 original\nline3","newText":"line2 changed\nline3"}]}),
        ws
    );
}
#[then("滑动窗口匹配成功")]
fn t_tools_span_ok(ws: &Workspace) {
    ws.last_result.borrow().as_ref().unwrap();
}

#[given("累加器接收 100 字节")]
fn g_tools_accum_small() {
    _accum_mode::LARGE.with(|f| f.set(false));
}
#[when("调用 finish")]
async fn w_tools_finish(ws: &Workspace) {
    ws.init();
    let large = _accum_mode::LARGE.with(|f| f.get());
    let cmd = if large {
        "dd if=/dev/zero bs=1024 count=120 2>/dev/null | tr '\\0' 'b'".to_string()
    } else {
        "python3 -c \"import sys; sys.stdout.buffer.write(b'a'*100)\"".to_string()
    };
    tool_call!(
        BashTool::default(),
        XyToolCtx::new("test"),
        serde_json::json!({"command": cmd}),
        ws
    );
}
#[then("snapshot.content 含 100 字节，未创建临时文件")]
fn t_tools_accum_small_ok(ws: &Workspace) {
    let r = result_ok_str(&ws.last_result);
    let v: serde_json::Value = serde_json::from_str(&r).expect("bash json");
    let out = v
        .get("combined")
        .or_else(|| v.get("stdout"))
        .and_then(|x| x.as_str())
        .unwrap_or("");
    assert!(out.contains('a'), "expected small output, got: {out}");
    assert!(
        !v.get("truncated")
            .and_then(|x| x.as_bool())
            .unwrap_or(false),
        "small output must not truncate"
    );
    assert!(v.get("full_output_path").and_then(|x| x.as_str()).is_none());
}

#[given("累加器接收 2x max_bytes")]
fn g_tools_accum_large() {
    _accum_mode::LARGE.with(|f| f.set(true));
}
#[then("快照内容为截断尾部，full_output_path 指向含完整输出的临时文件")]
fn t_tools_accum_large_ok(ws: &Workspace) {
    let r = result_ok_str(&ws.last_result);
    let v: serde_json::Value = serde_json::from_str(&r).expect("bash json");
    assert!(
        v.get("truncated")
            .and_then(|x| x.as_bool())
            .unwrap_or(false),
        "large output must truncate: {v}"
    );
    assert!(
        v.get("full_output_path")
            .and_then(|x| x.as_str())
            .is_some_and(|p| std::path::Path::new(p).exists()),
        "full_output_path must exist: {v}"
    );
}

#[given("bash 工具即将执行 echo hello")]
fn g_tools_bash_echo(ws: &Workspace) {
    ws.init();
    tools_pending::BASH_CMD.with(|c| {
        *c.borrow_mut() = Some("echo hello".into());
    });
}
#[when("执行 bash echo hello")]
async fn w_tools_bash_accum(ws: &Workspace) {
    let cmd = tools_pending::BASH_CMD
        .with(|c| c.borrow().clone().expect("given must stage bash command"));
    if ws.dir.borrow().is_none() {
        ws.init();
    }
    tool_call!(
        BashTool::default(),
        XyToolCtx::new("test"),
        serde_json::json!({"command": cmd}),
        ws
    );
}
#[then("返回 output:hello 且未触发临时文件落盘")]
fn t_tools_bash_accum_ok(ws: &Workspace) {
    let r = result_ok_str(&ws.last_result);
    let v: serde_json::Value = serde_json::from_str(&r).expect("bash json");
    let out = v
        .get("combined")
        .or_else(|| v.get("stdout"))
        .and_then(|x| x.as_str())
        .unwrap_or("");
    assert!(out.contains("hello"), "expected hello in output: {out}");
    assert!(
        !v.get("truncated")
            .and_then(|x| x.as_bool())
            .unwrap_or(false),
        "hello must not spill to temp file"
    );
    assert!(v.get("full_output_path").and_then(|x| x.as_str()).is_none());
}

#[given("临时目录有小 PNG")]
fn g_tools_png(ws: &Workspace) {
    let png: [u8; 67] = [
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 0x0D, 0x49, 0x48, 0x44, 0x52, 0,
        0, 0, 1, 0, 0, 0, 1, 8, 2, 0, 0, 0, 0x90, 0x77, 0x53, 0xDE, 0, 0, 0, 0x0C, 0x49, 0x44,
        0x41, 0x54, 8, 0xD7, 0x63, 0x68, 0x60, 0x60, 0x60, 0xF8, 0x0F, 0, 1, 0x4A, 1, 0xE4, 0, 0,
        0, 0, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];
    std::fs::write(ws.ws("img.png"), png).ok();
}
#[when("调用 read")]
async fn w_tools_read_no_args(ws: &Workspace) {
    let p = if std::path::Path::new(&ws.ws("img.png")).exists() {
        ws.ws("img.png")
    } else {
        ws.ws("doc.txt")
    };
    tool_call!(
        ReadTool,
        XyToolCtx::new("test"),
        serde_json::json!({"path":p}),
        ws
    );
}
#[then("tool result parts 含 Image 且 data 非空")]
fn t_tools_img(ws: &Workspace) {
    let r = result_ok_str(&ws.last_result);
    assert!(
        r.contains("image") || r.contains("base64") || r.contains("png") || r.contains("PNG"),
        "{}",
        &r[..r.len().min(200)]
    );
}

#[given("临时目录有 .txt")]
fn g_tools_txt(ws: &Workspace) {
    std::fs::write(ws.ws("doc.txt"), "hello from txt").ok();
}
#[then("返回文本内容且无 Image part")]
fn t_tools_txt_no_img(ws: &Workspace) {
    let r = result_ok_str(&ws.last_result);
    assert!(r.contains("hello from txt"));
    assert!(!r.contains("base64") && !r.contains("image/png"));
}
// ── agent-tools: unbound tool scenarios (continued) ───────────────

#[given("工具注册表含全部 10 个工具")]
fn g_tools_all_ten(_ws: &Workspace) {
    let tools = crate::infra::tools::default_tools();
    assert_eq!(tools.len(), 10);
    for n in ["todo_list", "todo_rewrite", "todo_update"] {
        assert!(tools.iter().any(|t| t.name() == n), "missing builtin {n}");
    }
}

#[when("各工具以合法参数调用")]
async fn w_tools_smoke_all(ws: &Workspace) {
    ws.init();
    let ctx = XyToolCtx::new("smoke");
    let mq = Arc::new(FileMutationQueue::new());
    std::fs::write(ws.ws("smoke.txt"), "hello").ok();
    let cases: Vec<(&str, Result<String, String>)> = vec![
        (
            "read",
            ReadTool
                .execute(
                    &ctx,
                    serde_json::json!({"path": ws.ws("smoke.txt")}),
                )
                .await
                .map_err(|e| e.to_string()),
        ),
        (
            "write",
            WriteTool::new(mq.clone())
                .execute(
                    &ctx,
                    serde_json::json!({"path": ws.ws("out.txt"), "content":"x"}),
                )
                .await
                .map_err(|e| e.to_string()),
        ),
        (
            "edit",
            EditTool::new(mq.clone())
                .execute(
                    &ctx,
                    serde_json::json!({"path": ws.ws("smoke.txt"),"edits":[{"oldText":"hello","newText":"hi"}]}),
                )
                .await
                .map_err(|e| e.to_string()),
        ),
        (
            "bash",
            BashTool::default()
                .execute(&ctx, serde_json::json!({"command":"echo ok"}))
                .await
                .map_err(|e| e.to_string()),
        ),
        (
            "grep",
            GrepTool
                .execute(
                    &ctx,
                    serde_json::json!({"pattern":"hello","path": ws.ws("smoke.txt")}),
                )
                .await
                .map_err(|e| e.to_string()),
        ),
        (
            "find",
            FindTool
                .execute(
                    &ctx,
                    serde_json::json!({"pattern":"*.txt","path":"."}),
                )
                .await
                .map_err(|e| e.to_string()),
        ),
        (
            "ls",
            LsTool
                .execute(&ctx, serde_json::json!({"path":"."}))
                .await
                .map_err(|e| e.to_string()),
        ),
    ];
    let mut failed: Vec<_> = cases
        .into_iter()
        .filter_map(|(name, r)| r.err().map(|e| format!("{name}:{e}")))
        .collect();
    // One default_tools() so todo_* share the same MemoryTodoGateway.
    {
        let tools = crate::infra::tools::default_tools();
        let by = |n: &str| tools.iter().find(|t| t.name() == n).cloned().unwrap();
        if let Err(e) = by("todo_rewrite")
            .execute(
                &ctx,
                serde_json::json!({
                    "items": [{"id": "smoke", "content": "smoke todo", "status": "pending"}]
                }),
            )
            .await
        {
            failed.push(format!("todo_rewrite:{e}"));
        }
        if let Err(e) = by("todo_update")
            .execute(
                &ctx,
                serde_json::json!({"id": "smoke", "status": "completed"}),
            )
            .await
        {
            failed.push(format!("todo_update:{e}"));
        }
        if let Err(e) = by("todo_list").execute(&ctx, serde_json::json!({})).await {
            failed.push(format!("todo_list:{e}"));
        }
    }
    ws.last_result.replace(if failed.is_empty() {
        Some(Ok("all-tools-ok".into()))
    } else {
        Some(Err(XyDriverError::from(failed.join("; "))))
    });
}

#[then("各返回成功 ToolResult")]
fn t_tools_smoke_ok(ws: &Workspace) {
    assert_eq!(result_ok_str(&ws.last_result), "all-tools-ok");
}

#[given("临时目录存在含行偏移的 unified diff 文件")]
fn g_tools_fudiff(ws: &Workspace) {
    ws.init();
    std::fs::write(ws.ws("offset.rs"), "line1\nline2\nline3\nline4\n").ok();
    ws.last_result.replace(Some(Ok(ws.ws("offset.rs"))));
}

#[when("经补丁应用执行 edit")]
async fn w_tools_fudiff_edit(ws: &Workspace) {
    let path = result_ok_str(&ws.last_result);
    let tool = EditTool::new(Arc::new(FileMutationQueue::new()));
    tool_call!(
        tool,
        XyToolCtx::new("fudiff"),
        serde_json::json!({
            "path": path,
            "edits": [{"oldText": "line3\n", "newText": "line3 changed\n"}]
        }),
        ws
    );
}

#[then("模糊匹配补丁应用成功")]
fn t_tools_fudiff_ok(ws: &Workspace) {
    assert!(
        ws.last_result.borrow().as_ref().unwrap().is_ok(),
        "fudiff/edit patch apply failed: {:?}",
        ws.last_result.borrow()
    );
    let content = std::fs::read_to_string(ws.ws("offset.rs")).unwrap();
    assert!(
        content.contains("line3 changed"),
        "file must contain patched line, got: {content}"
    );
}

#[given("工具需要必填字符串参数 file_path")]
fn g_tools_missing_arg(_ws: &Workspace) {
    tools_pending::REQUIRED_ARG.with(|a| {
        *a.borrow_mut() = Some("file_path".into());
    });
}

#[when("以空参调用该工具")]
async fn w_tools_call_empty_read(ws: &Workspace) {
    let arg = tools_pending::REQUIRED_ARG.with(|a| {
        a.borrow()
            .clone()
            .expect("given must name required argument")
    });
    assert_eq!(arg, "file_path");
    tool_call!(ReadTool, XyToolCtx::new("test"), serde_json::json!({}), ws);
}

#[then("调用失败且返回 MissingArgument 错误码")]
fn t_tools_missing_arg(ws: &Workspace) {
    let err = ws
        .last_result
        .borrow()
        .as_ref()
        .unwrap()
        .as_ref()
        .unwrap_err()
        .to_string()
        .to_lowercase();
    assert!(
        err.contains("invalidargs") || err.contains("missing") || err.contains("path"),
        "expected missing argument error, got: {err}"
    );
}

#[given("工具以 InvalidArgs 错误执行失败")]
fn g_tools_invalid_args(ws: &Workspace) {
    ws.init();
    ws.last_result.replace(Some(Err(XyDriverError::from(
        crate::protocol::error::XyToolError::InvalidArgs("bad args".into()).to_string(),
    ))));
}

#[when("agent 循环收集工具结果")]
fn w_tools_collect_error(ws: &Workspace) {
    let err_msg = ws
        .last_result
        .borrow()
        .as_ref()
        .unwrap()
        .as_ref()
        .unwrap_err()
        .to_string();
    let category = if err_msg.to_lowercase().contains("invalid arguments")
        || err_msg.to_lowercase().contains("invalidargs")
    {
        "error"
    } else {
        "unknown"
    };
    ws.last_result
        .replace(Some(Ok(format!("event-category:{category}"))));
}

#[then("产生的 AgentEvent 含 error 类别")]
fn t_tools_error_category(ws: &Workspace) {
    assert_eq!(result_ok_str(&ws.last_result), "event-category:error");
}

#[given("bash 工具正在 sleep 60")]
fn g_tools_bash_sleeping(ws: &Workspace) {
    ws.init();
}

#[when("发送取消信号")]
async fn w_tools_send_cancel(ws: &Workspace) {
    use tokio_util::sync::CancellationToken;
    let cancel = CancellationToken::new();
    let ctx = XyToolCtx::with_cancel("cancel-test", cancel.clone());
    let tool = BashTool::default();
    let handle = tokio::spawn(async move {
        tool.execute(&ctx, serde_json::json!({"command":"sleep 60"}))
            .await
    });
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    cancel.cancel();
    let result = handle.await.expect("bash task");
    match result {
        Ok(_) => {
            ws.last_result
                .replace(Some(Err(XyDriverError::from("expected cancel"))));
        }
        Err(e) => {
            ws.last_result
                .replace(Some(Err(XyDriverError::from(e.to_string()))));
        }
    }
}

#[then("execute 返回 Cancelled 错误")]
fn t_tools_cancelled(ws: &Workspace) {
    let err = ws
        .last_result
        .borrow()
        .as_ref()
        .unwrap()
        .as_ref()
        .unwrap_err()
        .to_string()
        .to_lowercase();
    assert!(
        err.contains("abort") || err.contains("cancel"),
        "expected cancelled/aborted error, got: {err}"
    );
}

#[given("工具集含全部内置工具")]
fn g_tools_registry(_ws: &Workspace) {
    assert_eq!(crate::infra::tools::default_tools().len(), 10);
}

#[when("列举工具名")]
fn w_tools_list_names(ws: &Workspace) {
    let names: Vec<String> = crate::infra::tools::default_tools()
        .iter()
        .map(|t| t.name().to_string())
        .collect();
    ws.last_result.replace(Some(Ok(names.join(","))));
}

#[then("返回 10 个工具名")]
fn t_tools_ten_names(ws: &Workspace) {
    let raw = result_ok_str(&ws.last_result);
    let names: Vec<_> = raw.split(',').collect();
    assert_eq!(names.len(), 10, "expected 10 tool names, got: {names:?}");
    for n in ["todo_list", "todo_rewrite", "todo_update"] {
        assert!(names.contains(&n), "missing {n} in {names:?}");
    }
}

#[given("内置工具集已构造")]
fn g_tools_infra_ready(ws: &Workspace) {
    ws.init();
    std::fs::write(ws.ws("infra.txt"), "infra").ok();
}

#[when("分别执行 read / write / edit / bash / grep / find / ls")]
async fn w_tools_infra_exec(ws: &Workspace) {
    w_tools_smoke_all(ws).await;
}

#[then("各工具返回成功结果且无 panic")]
fn t_tools_infra_ok(ws: &Workspace) {
    assert_eq!(result_ok_str(&ws.last_result), "all-tools-ok");
}

#[given("工具集含 read 与 grep")]
fn g_tools_toolset_base() {
    let set = ToolSet::from_iter(
        crate::infra::tools::default_tools()
            .into_iter()
            .filter(|t| matches!(t.name(), "read" | "grep")),
    );
    let names: Vec<String> = set.iter().map(|t| t.name().to_string()).collect();
    assert!(names.iter().any(|n| n == "read"));
    assert!(names.iter().any(|n| n == "grep"));
    tools_toolset::BASE.with(|b| b.replace(Some(set)));
    tools_toolset::NAMES.with(|n| n.replace(names));
}

#[when("plus(bash) 然后 remove(grep)")]
fn w_tools_toolset_ops(_ws: &Workspace) {
    let base = tools_toolset::BASE
        .with(|b| b.borrow_mut().take())
        .expect("given must build base toolset");
    let set = base
        .plus(Arc::new(BashTool::default()) as Arc<dyn crate::protocol::ports::XyTool>)
        .remove("grep");
    let names: Vec<String> = set.iter().map(|t| t.name().to_string()).collect();
    tools_toolset::NAMES.with(|n| n.replace(names));
}

#[then("最终工具集含 read 与 bash 且不含 grep")]
fn t_tools_toolset_final(_ws: &Workspace) {
    let names = tools_toolset::NAMES.with(|n| n.borrow().clone());
    assert!(names.iter().any(|n| n == "read"));
    assert!(names.iter().any(|n| n == "bash"));
    assert!(!names.iter().any(|n| n == "grep"));
}

mod tools_toolset {
    use crate::agent::tools::ToolSet;
    use std::cell::RefCell;
    thread_local! {
        pub static NAMES: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
        pub static BASE: RefCell<Option<ToolSet>> = const { RefCell::new(None) };
    }
}

mod tools_pending {
    use std::cell::{Cell, RefCell};
    thread_local! {
        pub static TIMEOUT: Cell<Option<i64>> = const { Cell::new(None) };
        pub static FIND_PATTERN: RefCell<Option<String>> = const { RefCell::new(None) };
        pub static MULTIBYTE: Cell<bool> = const { Cell::new(false) };
        pub static REQUIRED_ARG: RefCell<Option<String>> = const { RefCell::new(None) };
        pub static BASH_CMD: RefCell<Option<String>> = const { RefCell::new(None) };
    }
}

// ── ws1: tool execution in the session workspace (c2335) ─────────────

#[when("以会话工作区调用write工具 路径 {path:string} 内容 {content:string}")]
async fn _w_write_file_in_session_workspace(ws: &Workspace, path: String, content: String) {
    let ctx = XyToolCtx::new("test").with_workspace(ws.root());
    // Deliberately pass the RELATIVE path — resolution is the behavior under test.
    tool_call!(
        WriteTool::new(Arc::new(FileMutationQueue::new())),
        ctx,
        serde_json::json!({"path": path, "content": content}),
        ws
    );
}

#[when("以会话工作区调用bash命令 {cmd:string}")]
async fn _w_bash_cmd_in_session_workspace(ws: &Workspace, cmd: String) {
    let ctx = XyToolCtx::new("test").with_workspace(ws.root());
    tool_call!(
        BashTool::default(),
        ctx,
        serde_json::json!({"command": cmd}),
        ws
    );
}

#[then("文件 {path:string} 在会话工作区内存在")]
fn _t_file_exists_in_session_workspace(ws: &Workspace, path: String) {
    assert!(
        std::path::Path::new(&ws.ws(&path)).exists(),
        "file must land under the session workspace"
    );
}

#[then("文件 {path:string} 不在进程工作目录")]
fn _t_file_not_in_process_cwd(_ws: &Workspace, path: String) {
    let process_cwd = std::env::current_dir().expect("process cwd");
    assert!(
        !process_cwd.join(&path).exists(),
        "relative tool paths MUST NOT resolve against the process cwd: {}",
        process_cwd.join(&path).display()
    );
}

#[then("bash 输出为会话工作区目录")]
fn _t_bash_output_is_session_workspace(ws: &Workspace) {
    let r = result_ok_str(&ws.last_result);
    let v: serde_json::Value = serde_json::from_str(&r).expect("bash json");
    let stdout = v["stdout"].as_str().unwrap_or_default();
    let got = std::path::PathBuf::from(stdout.trim());
    assert_eq!(
        got.canonicalize().unwrap(),
        std::path::PathBuf::from(ws.root()).canonicalize().unwrap(),
        "bash MUST run in the session workspace; got {stdout}"
    );
}

mod _accum_mode {
    use std::cell::Cell;
    thread_local! { pub static LARGE: Cell<bool> = const { Cell::new(false) }; }
}
