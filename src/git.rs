use git_url_parse::GitUrl;
use std::{
    path::{Path, PathBuf}, process::Command
};
use tracing::info;

use crate::issue_tracker::UpstreamError;

/// Get the url of the `origin` remote.
pub fn guess_repo_url() -> Result<GitUrl, UpstreamError> {
    let cmd = Command::new("git")
        .args(["ls-remote", "--get-url", "origin"])
        .output()?;

    if !cmd.status.success() {
        return Err(UpstreamError::Other("Failed to get repo url".into()));
    }

    let url = String::from_utf8(cmd.stdout);

    Ok(GitUrl::parse(url?.trim())?)
}

pub fn get_repo_root() -> Result<PathBuf, UpstreamError> {
    let cmd = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()?;

    if cmd.status.success() {
        let path = String::from_utf8(cmd.stdout).unwrap();

        return Ok(PathBuf::from(path.trim()));
    }

    let target_git_dir_cmd = Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .output()?;
    if !target_git_dir_cmd.status.success() {
        return Err(UpstreamError::Other("Not in git repo".into()));
    }
    let target_git_dir_stdout = String::from_utf8(target_git_dir_cmd.stdout)?;
    let target_git_dir = Path::new(target_git_dir_stdout.trim_end());

    let base_git_dir_cmd = Command::new("git")
        .args(["rev-parse", "--git-common-dir"])
        .output()?;
    if !base_git_dir_cmd.status.success() {
        return Err(UpstreamError::Other("Not in git repo".into()));
    }
    let base_git_dir_stdout = String::from_utf8(base_git_dir_cmd.stdout)?;
    let base_git_dir = Path::new(base_git_dir_stdout.trim_end());
    info!("base_git_dir {}", base_git_dir.display());

    let worktrees_cmd = Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .output()?;
    if !worktrees_cmd.status.success() {
        return Err(UpstreamError::Other("Failed to get worktrees".into()));
    }

    let worktrees = String::from_utf8(worktrees_cmd.stdout)?;
    let mut base_worktree: Result<String, UpstreamError> = Err(UpstreamError::Other("Failed to find working directory".into()));

    for line in worktrees.lines() {
        if !line.starts_with("worktree ") {
            continue;
        }
        let path = &line["worktree ".len()..];
        let git_dir_cmd = Command::new("git")
            .args(["rev-parse", "--git-dir"])
            .current_dir(path)
            .output()?;
        let Ok(git_dir_stdout) = String::from_utf8(git_dir_cmd.stdout)
        else {
            continue;
        };
        let git_dir = Path::join(Path::new(path), git_dir_stdout.trim_end());
        info!("git_dir {}", git_dir.display());
        if git_dir == base_git_dir {
            base_worktree = Ok(path.into());
        } else if git_dir == target_git_dir {
            return Ok(PathBuf::from(path));
        }
    }

    Ok(PathBuf::from(base_worktree?))
}
