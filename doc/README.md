# Additional Project Documentation

Collection for additional information relevant to commit-lsp that does not fit into the main README.

## Ascii Cast in README

The "gif" inside of the README is an animated SVG.
It was created the following way:

1. Set the environment variable COMMIT_LSP_DEMO_FOLDER to a folder with some example files and used
   a debug build of commit-lsp.
2. Created `doc/autocomplete.cast` via [asciinema](https://asciinema.org/).
   `asciinema rec doc/autocomplete.cast`
3. Converted the ascii cast to an svg to allow embedding in the readme without any third party
   service dependency. Used [svg-term-cli](https://github.com/marionebl/svg-term-cli).
   `svg-term < doc/autocomplete.cast > doc/autocomplete.svg`

Note: svg-term doesn't support all terminals. In `kitty` the colors in neovim are lost.
