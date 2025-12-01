//! LLM Observatory CLI entry point.

fn main() {
    if let Err(e) = llm_observatory_cli::run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
