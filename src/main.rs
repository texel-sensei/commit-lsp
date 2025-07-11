use std::{fs::File, io::Read, process::ExitCode, sync::Mutex};

use clap::Parser as _;
use cli::Cli;
use git::guess_repo_url;
use healthcheck::{ComponentState, HealthReport, OptionExt, ResultExt};
use issue_tracker::IssueTracker;
use tracing::{info, trace};

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
            let user_config = config::User::load_default_file(&mut health);
            let repo_config = config::Repository::load_default_file(&mut health);
            trace!("Using config: {:?}", repo_config);
            let remote = initialize_issue_tracker(&user_config, &mut health);
            let analysis = analysis::State::new(repo_config);
            lsp::run_stdio(analysis, remote).await;
        }
        cli::Action::Lint { file } => {
            let mut health = HealthReport::silent();
            let mut text = String::new();
            File::open(&file)
                .unwrap()
                .read_to_string(&mut text)
                .unwrap();
            let repo_config = config::Repository::load_default_file(&mut health);
            return analyse_commit(repo_config, &text);
        }
        cli::Action::Checkhealth => {
            let mut health = HealthReport::new("commit-lsp");
            let user_config = config::User::load_default_file(&mut health);
            let _repo_config = config::Repository::load_default_file(&mut health);
            let remote = initialize_issue_tracker(&user_config, &mut health);

            if let Some(remote) = remote.report(&mut health, "Issue tracker initialized") {
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

fn analyse_commit(config: config::Repository, text: &str) -> ExitCode {
    let mut state = analysis::State::new(config);
    state.update_text(text);
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
    let remote_url = guess_repo_url();
    match &remote_url {
        Ok(url) => check.ok_with(url.to_string()),
        Err(_) => check.error("Failed to get remote url"),
    }
    let remote_url = remote_url.ok()?;

    info!("Using git url '{remote_url}'");
    let mut builder = issue_tracker::Builder::new(remote_url.clone());

    let remote_config = config.remote_specific_configuration(&remote_url.to_string());
    health.report(
        "Check for remote specific user config",
        if remote_config.is_some() {
            ComponentState::Ok(None)
        } else {
            ComponentState::Info("None found".into())
        },
    );

    let mut do_guess = true;

    if let Some(remote_config) = remote_config {
        let report = health.start("Check for configured issue tracker");

        if let Some(tracker) = remote_config.issue_tracker_type {
            report.ok_with(tracker.to_string());
            builder.tracker_type = Some(tracker);
            do_guess = false;
        } else {
            report.info("None configured, continue to guess");
        }
    }

    if do_guess {
        builder
            .tracker_type
            .report_with_some(health, "Guess used issue tracker");
    }

    if let Some(remote_config) = remote_config {
        builder.add_remote_config(health, remote_config.clone());
    }

    builder.build(health)
}
