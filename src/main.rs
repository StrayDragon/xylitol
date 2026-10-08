fn main() {
    // c2845: deep v3 session-tree serialization (`fory` codec recurses per tree
    // depth on `SessionTreeNode`) must not overflow tokio worker stacks — debug
    // frames are large, a ~185-level session tree already exceeded the default
    // (~2MiB) worker stack and aborted. Reserve a generous VIRTUAL stack (lazy
    // committed; RSS untouched until used) so any realistic tree round-trips on
    // both the served daemon and the TUI client (same binary/runtime).
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_stack_size(64 * 1024 * 1024)
        .build()
        .expect("Failed to create tokio runtime");
    if let Err(e) = rt.block_on(xylitol::run()) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
