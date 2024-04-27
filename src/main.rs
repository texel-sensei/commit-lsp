use std::{fs::File, io::Read, process::ExitCode};

use clap::Parser as _;
use cli::Cli;

pub mod analysis;
mod cli;
pub mod issue_tracker;
mod lsp;

pub mod git;

pub mod text_util;

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.action {
        cli::Action::Run => {
            lsp::run_stdio().await;
        }
        cli::Action::Check { file } => {
            let mut text = String::new();
            File::open(&file)
                .unwrap()
                .read_to_string(&mut text)
                .unwrap();
            return analyse_commit(&text);
        }
    }

    ExitCode::SUCCESS
}

fn analyse_commit(text: &str) -> ExitCode {
    let state = analysis::State::new(text);
    let diagnostics = state.all_diagnostics();

    for diag in &diagnostics {
        println!("{}", diag);
    }

    if diagnostics.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
