use clap::Parser;

use dolog::{Cli, run};

fn main() {
    let cli = Cli::parse();

    if let Err(error) = run(cli) {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
