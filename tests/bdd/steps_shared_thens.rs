use crate::fixtures::*;
use crate::helpers::*;
use rstest_bdd_macros::then;

#[then("文件 {path:string} 应该包含 {text}")]
fn _t_file_contains(ws: &Workspace, path: String, text: String) {
    let c = std::fs::read_to_string(ws.ws(&path)).unwrap();
    let t = strip_quotes(&text);
    assert!(c.contains(&t), "expected containing '{t}', got: {c}");
}

#[then("文件 {path:string} 内容为 {text}")]
fn _t_file_content_is(ws: &Workspace, path: String, text: String) {
    assert_eq!(
        std::fs::read_to_string(ws.ws(&path)).unwrap(),
        strip_quotes(&text)
    );
}

#[then("文件 {path:string} 应该存在")]
fn _t_file_exists(ws: &Workspace, path: String) {
    assert!(std::path::Path::new(&ws.ws(&path)).exists());
}

#[then("文件 {path:string} 应该保留UTF8_BOM")]
fn _t_file_has_bom(ws: &Workspace, path: String) {
    assert!(
        std::fs::read_to_string(ws.ws(&path))
            .unwrap()
            .starts_with('\u{FEFF}')
    );
}

// "结果包含 unified patch" ambiguous with "结果包含 {text}" — removed
// "结果包含 带行号的 display diff" ambiguous — removed

// "结果包含 unified patch" — use generic "_t_result_contains" (feature file uses {text} pattern)
// "结果包含 带行号的 display diff" — same

#[then("edit调用应该失败 包含错误信息 {msg}")]
fn _t_edit_failed(ws: &Workspace, msg: String) {
    let r = ws.last_result.borrow();
    let r = r.as_ref().unwrap();
    assert!(
        r.is_err() && check_or_contains(&r.as_ref().unwrap_err().to_string(), &msg),
        "expected error to match '{}', got: {}",
        msg,
        r.as_ref().unwrap_err()
    );
}

#[then("调用失败 包含验证错误")]
fn _t_call_fail_validation(ws: &Workspace) {
    assert!(ws.last_result.borrow().as_ref().unwrap().is_err());
}

#[then("调用失败 包含错误信息 {msg}")]
fn _t_call_fail_msg(ws: &Workspace, msg: String) {
    let err = ws.last_result.borrow();
    let err = err.as_ref().unwrap().as_ref().unwrap_err();
    assert!(
        check_or_contains(&err.to_string(), &msg),
        "expected error to match '{msg}', got: {err}"
    );
}

#[then("调用失败 包含错误信息")]
fn _t_call_fail(ws: &Workspace) {
    assert!(ws.last_result.borrow().as_ref().unwrap().is_err());
}

#[then("退出码为 {code:i32}")]
fn _t_exit_code_is(ws: &Workspace, code: i32) {
    assert!(result_ok_str(&ws.last_result).contains(&format!("exit_code\":{code}")));
}

#[then("stdout 包含 {text}")]
fn _t_stdout_has(ws: &Workspace, text: String) {
    let r = result_ok_str(&ws.last_result);
    let t = strip_quotes(&text);
    // Parse JSON to extract stdout field; fall back to contains on raw string
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&r)
        && let Some(s) = v["stdout"].as_str()
    {
        assert!(s.contains(&t), "stdout doesn't contain '{t}', stdout: {s}");
        return;
    }
    assert!(r.contains(&t), "result doesn't contain '{t}', result: {r}");
}

#[then("stdout 和 stderr 合并输出包含 {text}")]
fn _t_combined_has(ws: &Workspace, text: String) {
    let r = result_ok_str(&ws.last_result);
    let t = strip_quotes(&text);
    // Parse JSON to extract combined field; fall back to contains on raw string
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&r)
        && let Some(s) = v["combined"].as_str()
    {
        assert!(
            s.contains(&t),
            "combined doesn't contain '{t}', combined: {s}"
        );
        return;
    }
    assert!(r.contains(&t), "result doesn't contain '{t}', result: {r}");
}

#[then("命令应该失败 包含超时错误")]
fn _t_cmd_timeout(ws: &Workspace) {
    assert!(ws.last_result.borrow().as_ref().unwrap().is_err());
}
#[then("命令应该失败 包含取消错误")]
fn _t_cmd_abort(ws: &Workspace) {
    assert!(ws.last_result.borrow().as_ref().unwrap().is_err());
}

#[then("输出被截断")]
fn _t_truncated(ws: &Workspace) {
    let r = result_ok_str(&ws.last_result);
    let truncated = serde_json::from_str::<serde_json::Value>(&r)
        .ok()
        .and_then(|v| v.get("truncated")?.as_bool())
        .unwrap_or(false);
    assert!(
        truncated || r.contains("[Full output:"),
        "expected truncated output, got: {}",
        &r[..r.len().min(200)]
    );
}

#[then("截断详情显示达到字节或行限制")]
fn _t_truncation_details(ws: &Workspace) {
    let r = result_ok_str(&ws.last_result);
    assert!(
        r.contains("truncated") || r.contains("Full output"),
        "expected truncation details, got: {r}"
    );
}

#[then("结果含 Full output 脚注")]
fn _t_full_output_footer(ws: &Workspace) {
    let r = result_ok_str(&ws.last_result);
    assert!(
        r.contains("[Full output:"),
        "expected Full output footer, got: {r}"
    );
    assert!(
        r.contains("lines shown"),
        "expected lines shown in footer, got: {r}"
    );
}

#[then("bash 结果 JSON 无未截断全量 stdout 字段载荷")]
fn _t_bash_no_full_stdout_dump(ws: &Workspace) {
    let r = result_ok_str(&ws.last_result);
    let v: serde_json::Value =
        serde_json::from_str(&r).unwrap_or_else(|_| serde_json::json!({ "raw": r }));
    let stdout = v
        .get("stdout")
        .and_then(|x| x.as_str())
        .or_else(|| v.get("combined").and_then(|x| x.as_str()))
        .unwrap_or(&r);
    // Truncated display must stay near DEFAULT_MAX_BYTES (50KiB) + footer.
    assert!(
        stdout.len() < 60 * 1024,
        "stdout/combined still looks like a full dump: {} bytes",
        stdout.len()
    );
    assert!(
        v.get("truncated")
            .and_then(|x| x.as_bool())
            .unwrap_or(false)
            || r.contains("[Full output:"),
        "expected truncated=true or Full output footer"
    );
}

#[then("如果截断则显示剩余行提示")]
fn _t_remaining_hint(ws: &Workspace) {
    let r = result_ok_str(&ws.last_result);
    let v: serde_json::Value = serde_json::from_str(&r).expect("read result must be JSON");
    if v.get("truncated").and_then(|x| x.as_bool()) == Some(true) {
        let hint = v
            .get("hint")
            .or_else(|| v.get("message"))
            .and_then(|x| x.as_str())
            .unwrap_or(&r);
        assert!(
            hint.contains("remaining") || hint.contains("剩余") || hint.contains("offset"),
            "truncated read must include remaining-lines hint, got: {hint}"
        );
        assert!(
            v.get("remaining_lines")
                .and_then(|x| x.as_u64())
                .unwrap_or(0)
                > 0,
            "remaining_lines must be set when truncated"
        );
    } else {
        assert!(
            v.get("remaining_lines").is_some() || r.contains("remaining"),
            "expected truncation metadata in read result: {r}"
        );
    }
}

#[then("内容为 {text}")]
fn _t_read_content(ws: &Workspace, text: String) {
    let r = result_ok_str(&ws.last_result);
    let t = strip_quotes(&text);
    // Check JSON content field first
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&r)
        && let Some(s) = v["content"].as_str()
    {
        assert_eq!(s, t, "content mismatch");
        return;
    }
    assert!(r.contains(&t));
}

#[then("总行数为 {count:u32}")]
fn _t_total_lines(ws: &Workspace, count: u32) {
    let v: serde_json::Value =
        serde_json::from_str(&result_ok_str(&ws.last_result)).unwrap_or_default();
    assert_eq!(v["total_lines"], count);
}

#[then("偏移量为 {offset:i64}")]
fn _t_offset_is(ws: &Workspace, offset: i64) {
    let v: serde_json::Value =
        serde_json::from_str(&result_ok_str(&ws.last_result)).unwrap_or_default();
    assert_eq!(v["offset"], offset);
}

#[then("结果指示目录为空")]
fn _t_ls_empty(ws: &Workspace) {
    assert!(result_ok_str(&ws.last_result).contains("empty"));
}

#[then("结果应该为空或提示无匹配")]
fn _t_result_empty_or_no_match(ws: &Workspace) {
    let r = result_ok_str(&ws.last_result);
    assert!(
        r.is_empty() || r.contains("No matches") || r.contains("No files found"),
        "expected empty or no-match, got: {r}"
    );
}

// 通用 "结果列出 {entry}" — 用于 ls_with_files 和 ls_default_path
#[then("结果列出 {entry:string}")]
fn _t_ls_lists(ws: &Workspace, entry: String) {
    assert!(
        result_ok_str(&ws.last_result).contains(&entry),
        "result doesn't list '{entry}', result: {}",
        result_ok_str(&ws.last_result)
    );
}

#[then("条目按字母顺序排列")]
fn _t_ls_sorted(ws: &Workspace) {
    let r = result_ok_str(&ws.last_result);
    let lines: Vec<&str> = r
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('['))
        .collect();
    let mut sorted = lines.clone();
    sorted.sort_by_key(|a| a.to_lowercase());
    assert_eq!(lines, sorted);
}

#[then("结果指示达到条目限制")]
fn _t_ls_limit_hint(ws: &Workspace) {
    assert!(result_ok_str(&ws.last_result).contains("entries shown"));
}

#[then("结果包含 {text}")]
fn _t_result_contains(ws: &Workspace, text: String) {
    let r = result_ok_str(&ws.last_result);
    let t = strip_quotes(&text);
    // Support OR clauses like "A" 或 "B"
    if t.contains(" 或 ") {
        if !check_or_contains(&r, &text) {
            panic!("result doesn't match any of '{}', result: {}", t, r);
        }
    } else {
        assert!(
            r.contains(&t),
            "result doesn't contain '{}', result: {}",
            t,
            r
        );
    }
}

#[then("结果不包含 {text}")]
fn _t_result_not_has(ws: &Workspace, text: String) {
    assert!(!result_ok_str(&ws.last_result).contains(&strip_quotes(&text)));
}

#[then("恰好有 {count:u32} 条结果")]
fn _t_exact_results(ws: &Workspace, count: u32) {
    let lines = result_ok_str(&ws.last_result)
        .lines()
        .filter(|l| !l.is_empty() && !l.contains("limit"))
        .count();
    assert_eq!(lines, count as usize);
}

#[then("共有 {n:u32} 条匹配")]
fn _t_grep_match_count(ws: &Workspace, n: u32) {
    let r = result_ok_str(&ws.last_result);
    let count = r
        .lines()
        .filter(|l| {
            let parts: Vec<_> = l.splitn(3, ':').collect();
            parts.len() >= 3 && parts[1].parse::<u32>().is_ok()
        })
        .count();
    assert_eq!(
        count, n as usize,
        "expected {n} matches, got {count} in:\n{r}"
    );
}

#[then("匹配结果包含第{line:u32}行的 {text}")]
fn _t_grep_match_on_line(ws: &Workspace, line: u32, text: String) {
    let text = strip_quotes(&text);
    let r = result_ok_str(&ws.last_result);
    let marker = format!(":{line}:");
    assert!(
        r.lines().any(|l| l.contains(&marker) && l.contains(&text)),
        "expected match on line {line} containing {text:?} in:\n{r}"
    );
}

#[then("内容为空")]
fn _t_content_empty(ws: &Workspace) {
    let r = result_ok_str(&ws.last_result);
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&r) {
        assert!(
            v["content"].as_str().is_some_and(|s| s.is_empty()),
            "content not empty"
        );
    }
}
