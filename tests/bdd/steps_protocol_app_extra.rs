use crate::bdd::steps_domain_compaction::T4_PRINT_OUT;
use rstest_bdd_macros::{then, when};

#[when("对产品命令样例做线协议序列化与反序列化往返")]
pub(crate) fn w_t4_command_roundtrip() {
    use xylitol_protocol::Command;
    let samples = vec![
        Command::SwitchSession {
            session_path: "s1".into(),
        },
        Command::GetMessages {
            limit: None,
            before_entry_id: None,
        },
        Command::ExportJsonl {
            output_path: Some("/tmp/a.jsonl".into()),
        },
    ];
    let mut round = Vec::new();
    for c in samples {
        let json = serde_json::to_string(&c).expect("serialize");
        let back: Command = serde_json::from_str(&json).expect("deserialize");
        round.push(serde_json::to_string(&back).unwrap());
    }
    T4_PRINT_OUT.with(|o| *o.borrow_mut() = round.join("\n"));
}

#[then("命令往返保真且逐变体可反序列化")]
pub(crate) fn t_t4_command_roundtrip() {
    T4_PRINT_OUT.with(|o| {
        let o = o.borrow().clone();
        assert_eq!(o.lines().count(), 3, "c2827: 三变体均应往返：{o}");
        assert!(o.contains("s1"), "c2827: 载荷应保真：{o}");
    });
}
