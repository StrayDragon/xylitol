use super::super::*;
use xylitol_plugin_sdk::XySessionStore;
use xylitol_plugin_sdk::session::{
    EntryBase, MessageEntry, SESSION_VERSION, SealedIndex, SessionEntry, SessionHeader,
    SessionManifest, paginate_entries,
};

fn header(sid: &str) -> SessionEntry {
    SessionEntry::Header(SessionHeader {
        entry_type: "session".into(),
        version: SESSION_VERSION,
        id: format!("{sid}-header"),
        timestamp: 0,
        cwd: ".".into(),
        parent_session: None,
        fork_at_entry_id: None,
    })
}

fn message(id: &str) -> SessionEntry {
    SessionEntry::Message(MessageEntry {
        base: EntryBase {
            entry_type: "message".into(),
            id: id.into(),
            parent_id: None,
            timestamp: 1,
        },
        message: xylitol_plugin_sdk::session::fixture_message_json("user", id),
    })
}

async fn build_segmented_session(
    mgr: &SessionManager,
    sid: &str,
    sealed_sizes: &[usize],
    active_size: usize,
) -> Vec<SessionEntry> {
    mgr.create(sid, Some("."), None).await.unwrap();
    let mut all = Vec::new();
    let mut sealed = Vec::new();
    let mut generation = 0u64;
    for (segment_index, size) in sealed_sizes.iter().enumerate() {
        let mut entries: Vec<SessionEntry> = (0..*size)
            .map(|i| message(&format!("g{segment_index}e{i}")))
            .collect();
        if segment_index == 0 {
            entries.insert(0, header(sid));
        }
        let path = format!("segments/{generation:020}-sealed.jsonl");
        let mut segment = SessionManager::segment_descriptor(path, generation, &entries);
        segment.index_path = Some(SessionManager::sidecar_path_for(&segment.path));
        mgr.write_segment_atomically(sid, &segment, &entries)
            .await
            .unwrap();
        let index = SealedIndex::from_entries(&entries);
        mgr.write_sealed_index_atomically(
            sid,
            segment.index_path.as_ref().expect("sidecar"),
            &index,
        )
        .await
        .unwrap();
        sealed.push(segment);
        all.extend(entries);
        generation += 1;
    }

    let active_entries: Vec<SessionEntry> = (0..active_size)
        .map(|i| message(&format!("a{i}")))
        .collect();
    let active = SessionManager::segment_descriptor(
        format!("active-{generation}.jsonl"),
        generation,
        &active_entries,
    );
    mgr.write_segment_atomically(sid, &active, &active_entries)
        .await
        .unwrap();
    all.extend(active_entries);

    let mut manifest = SessionManifest::new(sid, active);
    manifest.sealed_segments = sealed;
    manifest.leaf_entry_id = all
        .last()
        .and_then(|entry| entry.entry_id().map(str::to_owned));
    mgr.write_manifest_atomically(sid, &manifest).await.unwrap();
    all
}

fn anchors(all: &[SessionEntry]) -> Vec<Option<String>> {
    vec![
        None,
        all.first()
            .and_then(|entry| entry.entry_id().map(str::to_owned)),
        all.get(39)
            .and_then(|entry| entry.entry_id().map(str::to_owned)),
        all.get(100)
            .and_then(|entry| entry.entry_id().map(str::to_owned)),
        all.get(149)
            .and_then(|entry| entry.entry_id().map(str::to_owned)),
        Some("missing".into()),
    ]
}

#[tokio::test]
async fn page_read_matches_full_slice() {
    let dir = tempfile::tempdir().unwrap();
    let mgr = SessionManager::new(dir.path().join("sessions"));
    let sid = "paged";
    let all = build_segmented_session(&mgr, sid, &[40, 60, 50], 30).await;
    let full = mgr.load(sid).await.unwrap();
    assert_eq!(full.len(), all.len());

    for limit in [None, Some(0), Some(1), Some(10), Some(50), Some(200)] {
        for anchor in anchors(&all) {
            let expected = paginate_entries(full.clone(), limit, anchor.as_deref());
            let actual = mgr
                .load_entries_page(sid, limit, anchor.as_deref())
                .await
                .unwrap();
            assert_eq!(
                actual, expected,
                "page read must equal the full slice for limit={limit:?} anchor={anchor:?}"
            );
        }
    }
}

#[tokio::test]
async fn newest_page_reads_only_the_active_segment() {
    let dir = tempfile::tempdir().unwrap();
    let mgr = SessionManager::new(dir.path().join("sessions"));
    let sid = "skip";
    build_segmented_session(&mgr, sid, &[40, 60, 50, 70, 30], 20).await;

    mgr.reset_segment_reads();
    let (page, has_more) = mgr.load_entries_page(sid, Some(10), None).await.unwrap();
    assert_eq!(page.len(), 10);
    assert!(has_more, "earlier entries remain on disk");
    assert_eq!(
        mgr.segment_reads(),
        1,
        "a page inside the active segment must not read sealed segments"
    );
}

#[tokio::test]
async fn anchored_page_skips_newer_segments() {
    let dir = tempfile::tempdir().unwrap();
    let mgr = SessionManager::new(dir.path().join("sessions"));
    let sid = "anchor-skip";
    let all = build_segmented_session(&mgr, sid, &[40, 60, 50, 70, 30], 20).await;
    let anchor = all[120].entry_id().unwrap().to_string();

    mgr.reset_segment_reads();
    let (page, has_more) = mgr
        .load_entries_page(sid, Some(10), Some(&anchor))
        .await
        .unwrap();
    assert_eq!(page.len(), 10);
    assert!(has_more, "entries before the anchor remain");
    assert!(
        mgr.segment_reads() <= 2,
        "an anchored page must skip newer segments; reads={}",
        mgr.segment_reads()
    );
}

#[tokio::test]
async fn unknown_anchor_returns_empty_page_without_scanning_sealed_segments() {
    let dir = tempfile::tempdir().unwrap();
    let mgr = SessionManager::new(dir.path().join("sessions"));
    let sid = "unknown-anchor";
    build_segmented_session(&mgr, sid, &[40, 60, 50], 30).await;

    mgr.reset_segment_reads();
    let (page, has_more) = mgr
        .load_entries_page(sid, Some(10), Some("missing"))
        .await
        .unwrap();
    assert!(page.is_empty(), "an unknown anchor must not guess a window");
    assert!(!has_more);
    assert_eq!(
        mgr.segment_reads(),
        1,
        "sidecars must let an unknown anchor skip sealed segments"
    );
}

#[tokio::test]
async fn full_page_reads_every_segment() {
    let dir = tempfile::tempdir().unwrap();
    let mgr = SessionManager::new(dir.path().join("sessions"));
    let sid = "full";
    let all = build_segmented_session(&mgr, sid, &[40, 60, 50], 30).await;

    mgr.reset_segment_reads();
    let (page, has_more) = mgr.load_entries_page(sid, None, None).await.unwrap();
    assert_eq!(page.len(), all.len());
    assert!(!has_more);
    assert_eq!(
        mgr.segment_reads(),
        4,
        "a full load still reads every segment"
    );
}
