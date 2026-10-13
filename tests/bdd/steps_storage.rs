//! app-tui-storage BDD 步骤（c2470）：纯存储层场景，tmpdir 直驱。

use crate::bdd::prelude::*;
use rstest::fixture;
use rstest_bdd_macros::{given, then, when};
use std::cell::Cell;
use std::path::PathBuf;
use std::sync::Arc;
use xylitol::app::tui::storage::{ClientStorage, KEY_EDITOR_HISTORY};

pub struct StorageBdd {
    pub root: RefCell<Option<PathBuf>>,
    pub a: RefCell<Option<Arc<ClientStorage>>>,
    pub b: RefCell<Option<Arc<ClientStorage>>>,
    pub invalid_rejected: Cell<bool>,
    pub corrupt_ok: Cell<bool>,
    pub concurrent_len: Cell<usize>,
    pub tmp_residue: Cell<bool>,
    pub lock_present: Cell<bool>,
}

#[fixture]
pub fn storage_bdd() -> StorageBdd {
    StorageBdd {
        root: RefCell::new(None),
        a: RefCell::new(None),
        b: RefCell::new(None),
        invalid_rejected: Cell::new(false),
        corrupt_ok: Cell::new(false),
        concurrent_len: Cell::new(0),
        tmp_residue: Cell::new(true),
        lock_present: Cell::new(false),
    }
}

fn tmp_root(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("xyl-bdd-storage-{tag}-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[when("以不同 channel 与非法段构造存储")]
fn w_partition(storage_bdd: &StorageBdd) {
    let root = tmp_root("partition");
    let a = Arc::new(ClientStorage::new(root.clone(), "default").unwrap());
    let b = Arc::new(ClientStorage::new(root.clone(), "dev").unwrap());
    storage_bdd
        .invalid_rejected
        .set(ClientStorage::new(root.clone(), "../escape").is_err());
    *storage_bdd.root.borrow_mut() = Some(root);
    *storage_bdd.a.borrow_mut() = Some(a);
    *storage_bdd.b.borrow_mut() = Some(b);
}

#[then("同根异桶互不可见且非法段被拒")]
fn t_partition(storage_bdd: &StorageBdd) {
    let a = storage_bdd.a.borrow().clone().expect("storage a");
    let b = storage_bdd.b.borrow().clone().expect("storage b");
    a.record_editor_prompt("from-default");
    assert_eq!(a.editor_history(), vec!["from-default".to_string()]);
    assert!(
        b.load::<Vec<String>>(KEY_EDITOR_HISTORY).is_none(),
        "异桶不可见"
    );
    assert!(storage_bdd.invalid_rejected.get(), "非法段须被拒");
}

#[given("存储通道就位")]
fn g_storage_ready(storage_bdd: &StorageBdd) {
    let a = Arc::new(ClientStorage::new(tmp_root("conc"), "default").unwrap());
    *storage_bdd.a.borrow_mut() = Some(a);
}

#[when("多线程并发更新同一 key")]
fn w_concurrent(storage_bdd: &StorageBdd) {
    let a = storage_bdd.a.borrow().clone().expect("storage");
    let handles: Vec<_> = (0..8)
        .map(|i| {
            let storage = a.clone();
            std::thread::spawn(move || {
                for j in 0..10 {
                    let text = format!("t{i}-{j}");
                    let _ = storage.update::<Vec<String>, _>(KEY_EDITOR_HISTORY, Vec::new(), |v| {
                        v.insert(0, text);
                    });
                }
            })
        })
        .collect();
    for h in handles {
        h.join().unwrap();
    }
    storage_bdd.concurrent_len.set(a.editor_history().len());
    let entries: Vec<String> = std::fs::read_dir(a.dir())
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    storage_bdd
        .tmp_residue
        .set(!entries.iter().any(|e| e.ends_with(".tmp")));
    storage_bdd
        .lock_present
        .set(entries.iter().any(|e| e == "editor-history.lock"));
}

#[then("并发条目全数在存且无临时文件残留")]
fn t_concurrent(storage_bdd: &StorageBdd) {
    assert_eq!(storage_bdd.concurrent_len.get(), 80, "并发写须全数落地");
    assert!(storage_bdd.tmp_residue.get(), "不得残留 .tmp");
    assert!(storage_bdd.lock_present.get(), "锁旁路文件须在");
}

#[when("key 文件被写坏")]
fn w_corrupt(storage_bdd: &StorageBdd) {
    let a = Arc::new(ClientStorage::new(tmp_root("corrupt"), "default").unwrap());
    std::fs::create_dir_all(a.dir()).unwrap();
    std::fs::write(a.dir().join("editor-history.json"), "{not json").unwrap();
    storage_bdd
        .corrupt_ok
        .set(a.load::<Vec<String>>(KEY_EDITOR_HISTORY).is_none());
    *storage_bdd.a.borrow_mut() = Some(a);
}

#[then("读回落空值由调用方初值兜底")]
fn t_corrupt(storage_bdd: &StorageBdd) {
    assert!(storage_bdd.corrupt_ok.get(), "损坏须回落 None 而非报错");
}

#[given("同 channel 双实例就位")]
fn g_dual(storage_bdd: &StorageBdd) {
    let root = tmp_root("dual");
    let a = Arc::new(ClientStorage::new(root.clone(), "default").unwrap());
    let b = Arc::new(ClientStorage::new(root, "default").unwrap());
    *storage_bdd.a.borrow_mut() = Some(a);
    *storage_bdd.b.borrow_mut() = Some(b);
}

#[when("A 与 B 先后写同一列表 key")]
fn w_dual_write(storage_bdd: &StorageBdd) {
    let a = storage_bdd.a.borrow().clone().expect("a");
    let b = storage_bdd.b.borrow().clone().expect("b");
    a.record_editor_prompt("from-a");
    b.record_editor_prompt("from-b");
}

#[then("双方重读均见两实例条目")]
fn t_dual_visible(storage_bdd: &StorageBdd) {
    let a = storage_bdd.a.borrow().clone().expect("a");
    let b = storage_bdd.b.borrow().clone().expect("b");
    for side in [&a, &b] {
        let v = side.editor_history();
        assert!(v.contains(&"from-a".to_string()), "{v:?}");
        assert!(v.contains(&"from-b".to_string()), "{v:?}");
        assert_eq!(v.len(), 2, "恰好两实例条目: {v:?}");
    }
}
