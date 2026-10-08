fn main() {
    // c2845: default worker stacks are correct — deep session trees no longer
    // traverse a recursive binary codec (session_tree rides RawOk / JSON, see
    // wire_v3.rs typed_payload), so no oversized stack reservation is needed.
    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    if let Err(e) = rt.block_on(xylitol::run()) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
