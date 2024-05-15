use std::{fs::File, io::Read as _};

use directories::ProjectDirs;
use serde::Deserialize;
use tracing::info;

use crate::healthcheck::{HealthReport, ResultExt};

#[derive(Deserialize, Debug, Clone, Default)]
pub struct User {
    pub remotes: Vec<Remote>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Remote {
    pub host: String,
    pub credentials_command: Vec<String>,
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
}
