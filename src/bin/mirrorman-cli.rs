use clap::Parser;
use mirrorman::cli;

fn main() {
    let cli = cli::Cli::parse();
    if let Err(e) = cli::execute(cli) {
        eprintln!("[!] Error: {e}");
        std::process::exit(1);
    }
}
