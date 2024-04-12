use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
pub struct Cli {
    #[clap(subcommand)]
    pub action: Action,
}

#[derive(Subcommand)]
pub enum Action {
    Run,
    Check{ file: PathBuf },
}
