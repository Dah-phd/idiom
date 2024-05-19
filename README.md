# Low config terminal IDE - designed for me

## Info

This is a very early version of the editor, currently LSP is supported and tested for rust (rust-analyzer) and partially for python with jedi-language-server. Thouse are set as defaults. You will need to supply the LSP servers on your own. And configure them in the .config file this could be done in the integrated terminal (CTRL + ~) with command %i load config.

In the integrated terminal you can also run %i help to get some besic idea what can be configured and most importantly the key map (very similar to VS code).

More documentation will not come very soon - current focus is developing basic features, although I would say the editor is quite usable.

## Tested platform

- Linux Fedora derivate (Nobara)

## TODO
- add state upone closing
- cleanup workspace render
- optimize the cache to reduce imput lag
- migrate to lsp_types 0.96
- implement proper len checks on info modals (currently some lines will be longer due to characters with greater width - non-breaking issue)
- improve error handling
- syntax on info modal
- optimize snippet insertion (current implementation is only functional)
- markdown rendering / editing
- !! more tests - till that point the goal has been to make the editor self-developing, so bugs can be easily found and structure crystalizes
- multi-cursor support
- runner passing arrow presses while process is running
- (backlog) semantic token styles for different lsp langs

## Initial target langs

- RUST
- Python
- JS/TS
- HTML/JSON/TOML/{YAML/YML}
