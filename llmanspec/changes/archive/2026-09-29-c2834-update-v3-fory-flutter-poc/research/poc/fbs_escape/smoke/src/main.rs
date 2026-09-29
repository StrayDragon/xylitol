mod wire {
    include!("../gen/rust/xy_wire_v3.rs");
}

fn main() {
    let notif = wire::ServerNotification {
        seq: 42,
        event: wire::Event::ErrorEvent(wire::ErrorEvent {
            kind: "Provider".into(),
            message: "upstream timeout".into(), // fdl 保留字,fbs 原名保留
        }),
        list: "todo-list-ref".into(),
        timestamp: 1727612345,
    };
    let bytes = notif.to_bytes().unwrap();
    let back = wire::ServerNotification::from_bytes(&bytes).unwrap();
    assert_eq!(notif, back);
    println!(
        "roundtrip OK ({} B): kind={} message={} list={} ts={}",
        bytes.len(),
        match &back.event {
            wire::Event::ErrorEvent(e) => (e.kind.clone(), e.message.clone()),
            _ => unreachable!(),
        }.0,
        match &back.event {
            wire::Event::ErrorEvent(e) => e.message.clone(),
            _ => unreachable!(),
        },
        back.list,
        back.timestamp
    );
}
