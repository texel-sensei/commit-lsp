# commit-lsp

Language Server for commit messages.

You got editor smarts for programming, editing config files and even writing prose.
Why stop at writing commit messages?

Commit-lsp brings linting and auto completion
for commit messages based on the conventional commit format.

![asciicast](./doc/autocomplete.svg)

## Planned feature set

The following features are implemented (✅), in work (🚧) or
planned but not yet implemented (❌):

- ❌ Style checking if the commit follows the conventional commit format
- ✅ Autocompletion for commit types and scopes with project specific config
    - Never guess again if your team uses `doc` or `docs` for documentation commits
- ✅ Autocompletion for work item references
    - commit-lsp queries for Issues/Tickets/Work Items assigned to your local git user
      and provides completion for those assigned to you
    - Support for:
        - ✅ Github
        - ✅ AzureDevOps
        - ✅ Gitlab
        - ❌ Jira

## Installation

### Homebrew

```bash
brew install texel-sensei/commit-lsp/commit-lsp
```

### From source

To build commit-lsp from source, make sure to have the latest rust toolchain installed and run:

    cargo install --locked commit-lsp

## Editor integration

Since commit-lsp uses the Language Server Protocol,
it should be compatible with every editor supporting it.

Commit-lsp is mainly tested with neovim.

### [neovim](https://neovim.io)

To integrate commit-lsp, copy the executable somewhere into PATH (or run `cargo install`) and add
the following into your `init.lua`:

```lua
vim.lsp.config("commit-lsp", {
    cmd = { "commit-lsp", "run" },
    root_markers = { '.git' },
    filetypes = { "gitcommit" }
})

if vim.fn.executable("commit-lsp") == 1 then
    vim.lsp.enable("commit-lsp")
end
```

### [helix](https://helix-editor.com/)

Copy the commit-lsp executable somewhere into PATH
and dd the following to your `languages.toml`:

```toml
[language-server.commit-lsp]
command = "commit-lsp"
args = ["run"]

[[language]]
name = "git-commit"
file-types = [{ glob = "COMMIT_EDITMSG" }, { glob = "MERGE_MSG" }, { glob = "COMMIT_EDITMSG-*.txt" }]
language-servers = [ "commit-lsp" ]
```

## Connecting to a remote issue tracker

The issue tracker integration is still very bare bones and work in progress.

Currently only AzureDevOps and Gitlab are supported.

The integration is controlled via a config file in the users home directory.
This config defines cli commands to provide credentials for the issue tracker.

The config file is located in the following places:

| OS      | location                                                          |
|---------|-------------------------------------------------------------------|
| Linux   | $XDG_CONFIG_HOME/commit-lsp/config.toml                           |
| Windows | %APPDATA%/texel/commit-lsp/config.toml                            |
| macOS   | $HOME/Library/Application Support/at.texel.commit-lsp/config.toml |

The config file contains a list of remotes.
The first remote where the host is a substring of the git remote URL will be picked.

For example:

```toml
[[remotes]]
host = "dev.azure.com"
credentials_command = ["pass", "show", "development/work/azure"]

[[remotes]]
host = "gitlab.example.com"
credentials_command = ["pass", "show", "development/hobby/gitlab"]

[[remotes]]
host = "gitlab.work.example.com"
credentials_command = ["pass", "show", "development/work/gitlab"]

[[remotes]]
host = "github.com"
credentials_command = ["gh", "auth", "token"]
```

If a host matches the origin URL of the repository origin,
then commit-lsp will run the command defined in `credentials_command`
to access the credentials.

The type of issue tracker is used will be guessed from the git remote URL.
But this kind of automatism might not work for all projects.
This automatism can be disabled by setting a specific tracker type via `issue_tracker_type`:

```toml
[[remotes]]
host = "git.work.example"
credentials_command = ["pass", "show", "development/work/gitlab"]
issue_tracker_type = "Gitlab"
```

Supported values are `Github`, `Gitlab` and `AzureDevOps`.

It is also possible to overwrite the URL of the remote via `issue_tracker_url`.
Both `issue_tracker_type` and `issue_tracker_url` are available
in both user config and repository config.

### AzureDevOps

The credentials command should print a Personal Access Token (PAT) to stdout.
The PAT must have Work Item `Read` access.

Issue numbers for autocompletion are taken from the "Recent Activity" category of the current project.
The AzureDevOps Organization and Project are parsed from the URL of the `origin` git remote.

### Gitlab

The credentials_command should print an access token to stdout with issue read access.
Autocompletion will use all open issues of the current project.
The project name and owner are parsed from the `origin` git remote URL.

### Github

The credentials_command should print an access token to stdout,
e.g. via `gh auth token`.
The project name and owner are parsed from the `origin` git remote URL.

If a token is given,
then issues assigned to the token owner are used in auto completion.
Without one issues assigned to the repo owner are completed.

## Troubleshooting

If autocompletion of issue numbers is not working,
run `commit-lsp checkhealth` in the repository.

This command runs several health checks and reports their status.
