fn main() {
    if let Err(err) = bear_rs::run() {
        eprintln!("error: {err:#}");
        std::process::exit(1);
    }
}
