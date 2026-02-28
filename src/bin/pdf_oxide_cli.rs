fn main() {
    if let Err(e) = pdf_oxide::cli::run() {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
