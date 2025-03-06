use std::{ffi::OsStr, fmt::Display, process::Command};

use git_url_parse::GitUrl;
use secure_string::SecureString;
use serde::Deserialize;
use tracing::warn;

use crate::{
    config::Remote,
    healthcheck::{HealthReport, ResultExt},
};

use super::{
    IssueTracker, IssueTrackerAdapter, azure::AzureDevops, demo::DemoAdapter, github::Github,
};

#[derive(Copy, Clone, Debug, Deserialize)]
pub enum IssueTrackerType {
    Demo,
    Gitlab,
    Github,
    AzureDevOps,
}

impl Display for IssueTrackerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                IssueTrackerType::Demo => "<DEMO>",
                IssueTrackerType::Gitlab => "Gitlab",
                IssueTrackerType::Github => "Github",
                IssueTrackerType::AzureDevOps => "Azure DevOps",
            }
        )
    }
}

impl IssueTrackerType {
    pub fn guess_from_url(url: GitUrl) -> Option<Self> {
        if cfg!(debug_assertions) && std::env::var("COMMIT_LSP_DEMO_FOLDER").is_ok() {
            return Some(Self::Demo);
        }

        match url.host.as_ref()?.as_str() {
            "ssh.dev.azure.com" | "dev.azure.com" => Some(Self::AzureDevOps),
            "github.com" => Some(Self::Github),
            host if host.contains("gitlab") => Some(Self::Gitlab),
            _ => None,
        }
    }
}

pub struct Builder {
    pub tracker_type: Option<IssueTrackerType>,
    url: GitUrl,
    credential_command: Option<Vec<String>>,
}

pub(super) struct TrackerConfig {
    pub url: GitUrl,
    pub secret: Option<SecureString>,
}

impl Builder {
    pub fn new(url: GitUrl) -> Self {
        Self {
            tracker_type: IssueTrackerType::guess_from_url(url.clone()),
            url,
            credential_command: None,
        }
    }

    pub fn add_remote_config(&mut self, health: &mut HealthReport, config: Remote) {
        self.credential_command = config.credentials_command;
        if let Some(url_override) = config.issue_tracker_url {
            let report = health.start("Apply url overwrite");
            let parsed = GitUrl::parse(&url_override);
            match parsed {
                Ok(url) => {
                    self.url = url.clone();
                    report.ok_with(url.trim_auth().to_string());
                }
                Err(err) => {
                    report.error(err.to_string());
                }
            }
            self.url = GitUrl::parse(&url_override).expect("URL override is not a valid url!");
        }
    }

    pub fn build(self, health: &mut HealthReport) -> Option<IssueTracker> {
        let mut secret = None;
        let report = health.start("Check for credentials command");
        if let Some(cmd) = self.credential_command {
            report.ok_with(cmd.join(" "));
            secret = get_credentials(&cmd).report(health, "Get credentials");
        } else {
            report.info("None configured")
        }

        let cfg = TrackerConfig {
            url: self.url,
            secret,
        };

        let adapter: Box<dyn IssueTrackerAdapter> = match self.tracker_type? {
            IssueTrackerType::Demo => Box::new(DemoAdapter::new(
                std::env::var("COMMIT_LSP_DEMO_FOLDER").unwrap().into(),
            )),
            IssueTrackerType::Gitlab => Box::new(super::gitlab::Gitlab::new(cfg)?),
            IssueTrackerType::Github => Box::new(Github::new(cfg)?),
            IssueTrackerType::AzureDevOps => Box::new(AzureDevops::new(cfg)?),
        };

        Some(IssueTracker::new(adapter))
    }
}

fn get_credentials(cmdline: &[impl AsRef<OsStr>]) -> Option<SecureString> {
    let pat = {
        let (cmd, args) = cmdline.split_first()?;

        let out = Command::new(cmd).args(args).output().unwrap();
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            let code = out.status.code();
            warn!(stderr, ?code, "Failed to execute credentials command!");
            return None;
        }
        String::from_utf8(out.stdout).unwrap().trim().into()
    };
    Some(pat)
}
