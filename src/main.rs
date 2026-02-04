use std::process;

fn main() {
    if let Err(e) = wavec::cli::run() {
        eprintln!("{}", e);
        wavec::cli::print_usage();
        process::exit(1);
    }
}
