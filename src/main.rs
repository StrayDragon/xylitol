fn main() {
    if let Err(e) = xylitol::run() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
