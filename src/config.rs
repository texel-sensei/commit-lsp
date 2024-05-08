use std::{fs::File, io::Read as _};

use directories::ProjectDirs;
use serde::Deserialize;
use tracing::info;

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
    pub fn load_default_file() -> Self {
        let proj_dir = ProjectDirs::from("at", "texel", "commit-lsp").unwrap();
        let dir = proj_dir.config_dir();

        let config_path = dir.join("config.toml");
        if !config_path.exists() {
            info!("Using default config");
            return Default::default();
        }
        let mut config_file = File::open(&config_path).unwrap();

        let mut text = String::new();
        config_file
            .read_to_string(&mut text)
            .expect("Failed to open config file!");

        info!("Loading config file '{path}'", path = config_path.display());
        toml::from_str(&text).expect("Failed to parse config!")
    }
}
