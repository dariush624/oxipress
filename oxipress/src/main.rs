fn main() {
    let include_drafts = std::env::args().any(|a| a == "--drafts");
    if let Err(e) = oxipress::build(oxipress::project_root(), include_drafts) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
