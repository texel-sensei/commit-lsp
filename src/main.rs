use std::{fs::File, io::Read, process::ExitCode, sync::Mutex};

use clap::Parser as _;
use cli::Cli;
use git::guess_repo_url;
use healthcheck::HealthReport;
use issue_tracker::IssueTracker;
use tracing::info;

pub mod analysis;
mod cli;
pub mod issue_tracker;
mod lsp;

pub mod healthcheck;

pub mod config;

pub mod git;

pub mod text_util;

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    if cfg!(debug_assertions) {
        let log_file = File::create("commit-lsp.log").expect("Failed to create log file");
        let subscriber = tracing_subscriber::fmt()
            .without_time()
            .pretty()
            .with_writer(Mutex::new(log_file))
            .finish();
        tracing::subscriber::set_global_default(subscriber).unwrap();
    }

    match cli.action {
        cli::Action::Run => {
            let mut health = HealthReport::silent();
            let config = config::User::load_default_file(&mut health);
            let remote = initialize_issue_tracker(&config, &mut health);
            lsp::run_stdio(remote).await;
        }
        cli::Action::Lint { file } => {
            let mut text = String::new();
            File::open(&file)
                .unwrap()
                .read_to_string(&mut text)
                .unwrap();
            return analyse_commit(&text);
        }
        cli::Action::Checkhealth => {
            let mut health = HealthReport::new("commit-lsp");
            let config = config::User::load_default_file(&mut health);
            let remote = initialize_issue_tracker(&config, &mut health);

            if let Some(remote) = remote {
                let check = health.start("request tickets");
                match remote.request_ticket_information().await {
                    Ok(tickets) if !tickets.is_empty() => {
                        let example = tickets.first().unwrap();
                        check.ok_with(format!(
                            "Example ticket: #{} '{}'",
                            example.id(),
                            example.title()
                        ));
                    }
                    Ok(_) => {
                        check.warn("Got empty list of tickets");
                    }
                    Err(e) => {
                        check.error(e.to_string());
                    }
                }
            }
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

fn initialize_issue_tracker(
    config: &config::User,
    health: &mut HealthReport,
) -> Option<IssueTracker> {
    health.set_context("Issue Tracker");

    let check = health.start("retrieve repo url");
    let url_info = guess_repo_url();
    match &url_info {
        Some(url) => check.ok_with(format!("Got '{url}'")),
        None => check.error("Failed to get remote url"),
    }
    let url_info = url_info?;

    info!("Using git url '{url_info}'");
    IssueTracker::guess_from_remote(url_info, &config, health)
}
