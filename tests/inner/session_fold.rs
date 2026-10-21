use xylitol_plugin_sdk::message::{AgentMessage, AgentPart, BashExecutionStatus, XyLlmMessage};
use xylitol_plugin_sdk::session::{
    SessionEntry, bash_execution_message_entry, done_bash_ids, fold_interrupted_bash_rows,
};

fn bash_row(
    bash_id: &str,
    command: &str,
    status: BashExecutionStatus,
    exclude: bool,
) -> SessionEntry {
    bash_execution_message_entry(
        bash_id,
        command,
        "",
        Some(0),
        false,
        false,
        None,
        exclude,
        status,
    )
}

fn mapped(entries: &[SessionEntry]) -> Vec<AgentMessage> {
    entries
        .iter()
        .filter_map(|e| e.as_agent_message())
        .collect()
}

fn user_text_of(msg: &XyLlmMessage) -> Option<String> {
    match msg {
        XyLlmMessage::UserMessage { content, .. } => content.iter().find_map(|p| match p {
            AgentPart::Text { text } => Some(text.clone()),
            _ => None,
        }),
        _ => None,
    }
}

#[test]
fn orphan_running_projects_interrupted_notice() {
    let entries = vec![bash_row("b1", "serve", BashExecutionStatus::Running, false)];
    let done = done_bash_ids(&entries);
    let folded = fold_interrupted_bash_rows(mapped(&entries), &done);
    let rows = xylitol_runtime::agent::llm_project::project_for_llm(&folded);
    let texts: Vec<_> = rows.iter().filter_map(user_text_of).collect();
    assert_eq!(
        texts,
        vec!["[interrupted] $ serve".to_string()],
        "orphan running MUST fold into the pinned interrupted line: {texts:?}"
    );
}

#[test]
fn paired_running_stays_hidden_and_done_folds() {
    let entries = vec![
        bash_row("b1", "serve", BashExecutionStatus::Running, false),
        bash_row("b1", "serve", BashExecutionStatus::Done, false),
    ];
    let done = done_bash_ids(&entries);
    let folded = fold_interrupted_bash_rows(mapped(&entries), &done);
    let rows = xylitol_runtime::agent::llm_project::project_for_llm(&folded);
    let texts: Vec<_> = rows.iter().filter_map(user_text_of).collect();
    assert!(
        texts.iter().all(|t| !t.contains("[interrupted]")),
        "paired running MUST NOT project interrupted: {texts:?}"
    );
    assert!(
        texts.iter().any(|t| t.contains("$ serve")),
        "done row keeps the bash fold: {texts:?}"
    );
}

#[test]
fn excluded_orphan_never_projects() {
    let entries = vec![bash_row("b1", "clean", BashExecutionStatus::Running, true)];
    assert!(
        mapped(&entries).is_empty(),
        "excluded rows are dropped before the fold (as_agent_message)"
    );
    let folded = fold_interrupted_bash_rows(mapped(&entries), &done_bash_ids(&entries));
    assert!(
        xylitol_runtime::agent::llm_project::project_for_llm(&folded).is_empty(),
        "`!!` orphan MUST NOT project, interrupted included"
    );
}

#[test]
fn repeated_folds_are_byte_stable() {
    let entries = vec![
        bash_row("b1", "serve", BashExecutionStatus::Running, false),
        bash_row("b1", "serve", BashExecutionStatus::Done, false),
        bash_row("b2", "serve", BashExecutionStatus::Running, false),
    ];
    let done = done_bash_ids(&entries);
    let texts = |rows: &Vec<XyLlmMessage>| -> Vec<String> {
        rows.iter().filter_map(user_text_of).collect()
    };
    let first = texts(&xylitol_runtime::agent::llm_project::project_for_llm(
        &fold_interrupted_bash_rows(mapped(&entries), &done),
    ));
    let second = texts(&xylitol_runtime::agent::llm_project::project_for_llm(
        &fold_interrupted_bash_rows(mapped(&entries), &done),
    ));
    assert_eq!(first, second, "repeated context builds MUST be byte-stable");
}

#[test]
fn done_off_leaf_path_still_suppresses() {
    let on_path = vec![bash_row("b1", "serve", BashExecutionStatus::Running, false)];
    let whole_session = vec![
        bash_row("b1", "serve", BashExecutionStatus::Running, false),
        bash_row("b1", "serve", BashExecutionStatus::Done, false),
    ];
    let done = done_bash_ids(&whole_session);
    let folded = fold_interrupted_bash_rows(mapped(&on_path), &done);
    let rows = xylitol_runtime::agent::llm_project::project_for_llm(&folded);
    assert!(
        rows.is_empty(),
        "done off the leaf path MUST suppress the interrupted notice: {rows:?}"
    );
}
