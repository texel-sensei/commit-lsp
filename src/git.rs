use git_url_parse::GitUrl;
use std::process::Command;

/// Get the url of the `origin` remote.
pub fn guess_repo_url() -> Option<GitUrl> {
    let url = Command::new("git")
        .args(["ls-remote", "--get-url", "origin"])
        .output()
        .unwrap()
        .stdout;
    let url = String::from_utf8(url).unwrap();

    GitUrl::parse(url.trim()).ok()
}
