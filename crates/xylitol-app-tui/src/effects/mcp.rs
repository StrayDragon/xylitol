use serde_json::Value;
use xylitol_plugin_sdk::mcp::McpResourceContext;
use xylitol_tui::Terminal;

use crate::driver::XyDriver;
use crate::host::HostSession;
use crate::mcp_attach::{
    McpPromptTarget, PendingMcpAttachments, catalog_find_resource, prompt_arg_fields,
    prompt_result_text,
};

pub(super) async fn resolve_contexts<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn XyDriver,
    attachments: &PendingMcpAttachments,
) -> Option<Vec<McpResourceContext>> {
    if attachments.is_empty() {
        return Some(Vec::new());
    }
    if !attachments.tokens.is_empty() {
        let catalog = match driver.mcp_catalog().await {
            Ok(catalog) => catalog,
            Err(e) => {
                e.log_failure("tui.mcp_catalog");
                session
                    .push_scroll_notice(format!("mcp catalog unavailable: {e} — message not sent"));
                return None;
            }
        };
        let unknown: Vec<String> = attachments
            .tokens
            .iter()
            .filter(|r| catalog_find_resource(&catalog.resources, &r.server, &r.uri).is_none())
            .map(|r| format!("{}/{}", r.server, r.uri))
            .collect();
        if !unknown.is_empty() {
            session.push_scroll_notice(format!(
                "unknown mcp resource: {} — message not sent (remove the @mcp:\"…\" token or attach via /mcp resource)",
                unknown.join(", ")
            ));
            return None;
        }
    }
    let mut contexts = Vec::new();
    for r in attachments.combined() {
        match driver.mcp_resource_read(&r.server, &r.uri).await {
            Ok(content) => {
                if content.binary {
                    session.push_scroll_notice(format!(
                        "mcp resource is binary and cannot attach: {}/{} — message not sent",
                        r.server, r.uri
                    ));
                    return None;
                }
                contexts.push(McpResourceContext {
                    server: content.server,
                    uri: content.uri,
                    mime: content.mime,
                    text: content.text,
                    truncated: content.truncated,
                });
            }
            Err(e) => {
                e.log_failure("tui.mcp_resource_read");
                session.push_scroll_notice(format!(
                    "mcp resource read failed: {}/{}: {e} — message not sent (remove it from the attach tray or retry)",
                    r.server, r.uri
                ));
                return None;
            }
        }
    }
    Some(contexts)
}

pub(super) async fn drain<T: Terminal>(session: &mut HostSession<T>, driver: &mut dyn XyDriver) {
    resource_add(session).await;
    prompt_select(session, driver).await;
    prompt_args(session, driver).await;
}

async fn resource_add<T: Terminal>(session: &mut HostSession<T>) {
    let Some(r) = session.take_pending_mcp_resource_add() else {
        return;
    };
    session.close_mcp_slot();
    if session.mcp_tray_add(&r.server, &r.uri) {
        session.push_toast_info_notice(format!("Attached {}/{}", r.server, r.uri));
    } else {
        session.push_toast_info_notice(format!("Already attached {}/{}", r.server, r.uri));
    }
    let _ = session.render_now();
}

async fn fetch_catalog<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn XyDriver,
) -> Option<xylitol_plugin_sdk::mcp::McpCatalog> {
    match driver.mcp_catalog().await {
        Ok(catalog) => Some(catalog),
        Err(e) => {
            e.log_failure("tui.mcp_catalog");
            session.push_scroll_notice(format!("mcp catalog unavailable: {e}"));
            let _ = session.render_now();
            None
        }
    }
}

async fn fetch_and_insert_prompt<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn XyDriver,
    server: &str,
    name: &str,
    arguments: Value,
) {
    match driver.mcp_prompt_get(server, name, arguments).await {
        Ok(value) => {
            let text = prompt_result_text(&value);
            if !text.trim().is_empty() {
                session.insert_editor_prompt_text(&text);
            }
            session.push_scroll_notice(format!(
                "prompt {server}/{name} inserted into the editor (not sent)"
            ));
        }
        Err(e) => {
            e.log_failure("tui.mcp_prompt_get");
            session.push_scroll_notice(format!("mcp prompt get failed: {server}/{name}: {e}"));
        }
    }
    let _ = session.render_now();
}

async fn prompt_select<T: Terminal>(session: &mut HostSession<T>, driver: &mut dyn XyDriver) {
    let Some((server, name)) = session.take_pending_mcp_prompt_select() else {
        return;
    };
    let Some(catalog) = fetch_catalog(session, driver).await else {
        return;
    };
    let Some(entry) = catalog
        .prompts
        .iter()
        .find(|e| e.server == server && e.name == name)
    else {
        session.push_scroll_notice(format!("mcp prompt not found: {server}/{name}"));
        let _ = session.render_now();
        return;
    };
    let fields = prompt_arg_fields(entry.schema.as_ref());
    if fields.is_empty() {
        session.close_mcp_slot();
        fetch_and_insert_prompt(session, driver, &server, &name, serde_json::json!({})).await;
        return;
    }
    session.mount_mcp_prompt_args(McpPromptTarget {
        server,
        name,
        fields,
    });
    let _ = session.render_now();
}

async fn prompt_args<T: Terminal>(session: &mut HostSession<T>, driver: &mut dyn XyDriver) {
    let Some((target, json)) = session.take_mcp_prompt_args_result() else {
        return;
    };
    let Ok(value) = serde_json::from_str::<Value>(&json) else {
        session.push_scroll_notice(format!(
            "prompt {}/{} form returned unreadable answer",
            target.server, target.name
        ));
        let _ = session.render_now();
        return;
    };
    if value.get("status").and_then(Value::as_str) != Some("answered") {
        session.push_scroll_notice(format!(
            "prompt {}/{} form cancelled",
            target.server, target.name
        ));
        let _ = session.render_now();
        return;
    }
    let answers = value
        .get("answers")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let answer_for = |field: &str| -> String {
        answers
            .iter()
            .find(|a| a.get("id").and_then(Value::as_str) == Some(field))
            .and_then(|a| a.get("values").and_then(Value::as_array))
            .and_then(|values| values.first())
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string()
    };
    let mut arguments = serde_json::Map::new();
    let mut missing = Vec::new();
    for field in &target.fields {
        let answer = answer_for(&field.name);
        if answer.is_empty() {
            if field.required {
                missing.push(field.name.clone());
            }
            continue;
        }
        arguments.insert(field.name.clone(), Value::String(answer));
    }
    if !missing.is_empty() {
        session.push_scroll_notice(format!(
            "prompt {}/{} missing required argument: {} — form reopened",
            target.server,
            target.name,
            missing.join(", ")
        ));
        session.mount_mcp_prompt_args(target);
        let _ = session.render_now();
        return;
    }
    fetch_and_insert_prompt(
        session,
        driver,
        &target.server,
        &target.name,
        Value::Object(arguments),
    )
    .await;
}
