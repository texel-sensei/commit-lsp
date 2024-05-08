use std::{fs::File, io::Read, process::ExitCode};

use clap::Parser as _;
use cli::Cli;
use git::guess_repo_url;
use issue_tracker::IssueTracker;

pub mod analysis;
mod cli;
pub mod issue_tracker;
mod lsp;

pub mod config;

pub mod git;

pub mod text_util;

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    let config = config::User::load_default_file();
    let remote = initialize_issue_tracker(&config);

    match cli.action {
        cli::Action::Run => {
            lsp::run_stdio(remote).await;
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

fn initialize_issue_tracker(config: &config::User) -> Option<IssueTracker> {
    let url_info = guess_repo_url()?;
    IssueTracker::guess_from_remote(url_info, &config)
}
