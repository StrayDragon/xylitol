use crate::bdd::prelude::*;
use rstest::fixture;
use rstest_bdd_macros::{then, when};

pub struct T2SchemaBdd {
    pub schemas: RefCell<Vec<(String, serde_json::Value)>>,
}

#[fixture]
pub fn t2_schema_bdd() -> T2SchemaBdd {
    T2SchemaBdd {
        schemas: RefCell::new(Vec::new()),
    }
}

#[when("检查内置工具的参数 schema")]
pub(crate) fn w_t2_check_schemas(t2_schema_bdd: &T2SchemaBdd) {
    let schemas = xylitol_official_plugin_tools::default_tools()
        .iter()
        .map(|t| (t.name().to_string(), t.parameters_schema()))
        .collect();
    *t2_schema_bdd.schemas.borrow_mut() = schemas;
}

#[then("grep 与 find 的 schema 含可选 timeout")]
pub(crate) fn t_t2_search_timeout(t2_schema_bdd: &T2SchemaBdd) {
    let schemas = t2_schema_bdd.schemas.borrow();
    for name in ["grep", "find"] {
        let schema = &schemas
            .iter()
            .find(|(n, _)| n == name)
            .unwrap_or_else(|| panic!("c2827: tool {name} must exist"))
            .1;
        let props = &schema["properties"];
        assert!(
            props.get("timeout").is_some(),
            "c2827: {name} schema 应含可选 timeout：{schema}"
        );
    }
}

#[then("文件类四工具的 schema 不含 per-call timeout 参数")]
pub(crate) fn t_t2_fs_no_timeout(t2_schema_bdd: &T2SchemaBdd) {
    let schemas = t2_schema_bdd.schemas.borrow();
    for name in ["read", "write", "edit", "ls"] {
        let schema = &schemas
            .iter()
            .find(|(n, _)| n == name)
            .unwrap_or_else(|| panic!("c2827: tool {name} must exist"))
            .1;
        let props = schema["properties"].as_object().expect("properties");
        assert!(
            !props.contains_key("timeout"),
            "c2827: {name} schema MUST NOT 暴露 per-call timeout：{schema}"
        );
    }
}
