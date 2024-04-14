# commit-lsp

Language Server for git commit messages.


You got editor smarts for programming, editing config files and even writing prose.
Why stop at writing commit messages?

Commit-lsp brings linting and auto completion
for commit messages based on the conventional commit format.

## Planned feature set

The following features are implemented (✅), in work (🚧) or
planned but not yet implemented (❌):

- ❌ Style checking if the commit follows the conventional commit format
- ❌ Autocompletion for commit types and scopes with project specific config
    - Never guess again if your team uses `doc` or `docs` for documentation commits
- ❌ Autocompletion for work item references
    - commit-lsp queries for Issues/Tickets/Work Items assigned to your local git user
      and provides completion for those assigned to you
    - Support for:
        - ❌ github
        - ❌ AzureDevOps
        - ❌ gitlab

## Editor integration

Since commit-lsp uses the Language Server Protocol,
it should be compatible with every editor supporting it.

Commit-lsp is tested with neovim.
To integrate commit-lsp, copy the executable somewhere into PATH (or run `cargo install`) and add
the following into your `init.lua`:

```lua
if vim.fn.executable("commit-lsp") == 1 then
	vim.api.nvim_create_autocmd(
		"FileType",
		{
			group=vim.api.nvim_create_augroup("CommitLspStart", {}),
			pattern="gitcommit",
			callback=function()
				local client = vim.lsp.start_client {
					cmd = { "commit-lsp", "run" },
				}
				vim.lsp.buf_attach_client(0, client)
			end
		}
	)
end

```
