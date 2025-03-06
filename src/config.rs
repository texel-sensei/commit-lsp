use std::{fs::File, io::Read as _};

use directories::ProjectDirs;
use serde::Deserialize;
use tracing::info;

use crate::{
    git::get_repo_root,
    healthcheck::{HealthReport, ResultExt},
    issue_tracker::IssueTrackerType,
};

#[derive(Deserialize, Debug, Clone, Default)]
pub struct User {
    pub remotes: Vec<Remote>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Remote {
    pub host: String,
    pub credentials_command: Option<Vec<String>>,

    pub issue_tracker_type: Option<IssueTrackerType>,
    pub issue_tracker_url: Option<String>,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct Repository {
    pub types: Vec<CommitElementDefinition>,
    pub scopes: Vec<CommitElementDefinition>,

    pub issue_tracker_type: Option<IssueTrackerType>,
    pub issue_tracker_url: Option<String>,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct CommitElementDefinition {
    pub name: String,
    pub summary: String,
    pub description: String,
}

impl User {
    pub fn load_default_file(health: &mut HealthReport) -> Self {
        health.set_context("User Configuration");

        let proj_dir = ProjectDirs::from("at", "texel", "commit-lsp").unwrap();
        let dir = proj_dir.config_dir();

        let config_path = dir.join("config.toml");

        let check = health.start(format!("open config file ('{}')", config_path.display()));
        if !config_path.exists() {
            info!("Using default config");
            check.info("File does not exist, using default config.");
            return Default::default();
        }
        let text = (|| {
            let mut config_file = File::open(&config_path)?;

            let mut text = String::new();
            config_file.read_to_string(&mut text)?;

            std::io::Result::Ok(text)
        })()
        .finish_check(check)
        .unwrap();

        info!("Loading config file '{path}'", path = config_path.display());
        toml::from_str(&text)
            .report(health, "parse config")
            .expect("Failed to parse config!")
    }

    /// Finds remote specific configuration for a given url, if it exists.
    ///
    /// Matching happens via a simple substring match of the `host` setting.
    /// The first host that is contained in the url is picked.
    pub fn remote_specific_configuration(&self, url: &str) -> Option<&Remote> {
        self.remotes.iter().find(|r| url.contains(&r.host))
    }
}

impl Repository {
    pub fn load_default_file(health: &mut HealthReport) -> Self {
        health.set_context("Repository Configuration");

        let root_folder = get_repo_root()
            .report(health, "is inside git repository")
            .unwrap();

        let config_path = root_folder.join(".commit-lsp.toml");

        let check = health.start(format!("open config file ('{}')", config_path.display()));
        if !config_path.exists() {
            info!("Using default config");
            check.info("File does not exist, using default config.");
            return Default::default();
        }
        let text = (|| {
            let mut config_file = File::open(&config_path)?;

            let mut text = String::new();
            config_file.read_to_string(&mut text)?;

            std::io::Result::Ok(text)
        })()
        .finish_check(check)
        .unwrap();

        info!("Loading config file '{path}'", path = config_path.display());
        toml::from_str(&text)
            .report(health, "parse config")
            .expect("Failed to parse config!")
    }
}
