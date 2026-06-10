fn main() {
    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    if let Err(e) = rt.block_on(xylitol::run()) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
