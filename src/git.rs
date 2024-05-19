use git_url_parse::GitUrl;
use std::{path::PathBuf, process::Command};

/// Get the url of the `origin` remote.
pub fn guess_repo_url() -> Option<GitUrl> {
    let cmd = Command::new("git")
        .args(["ls-remote", "--get-url", "origin"])
        .output()
        .unwrap();

    if !cmd.status.success() {
        return None;
    }

    let url = String::from_utf8(cmd.stdout).unwrap();

    GitUrl::parse(url.trim()).ok()
}

pub fn get_repo_root() -> Option<PathBuf> {
    let cmd = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .unwrap();

    if !cmd.status.success() {
        return None;
    }

    let path = String::from_utf8(cmd.stdout).unwrap();

    Some(PathBuf::from(path.trim()))
}
