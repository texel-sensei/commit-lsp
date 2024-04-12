use std::{fs::File, io::Read};

use clap::Parser as _;
use cli::Cli;

pub mod analysis;
mod cli;
mod lsp;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.action {
        cli::Action::Run => {
            lsp::run_stdio().await;
        }
        cli::Action::Check{file} => {
            let mut text = String::new();
            File::open(&file).unwrap().read_to_string(&mut text).unwrap();
            analyse_commit(&text);
        },
    }
}

fn analyse_commit(text: &str) {
    let state = analysis::State::new(text);
    for diag in state.all_diagnostics() {
        println!("{}", diag);
    }
}
