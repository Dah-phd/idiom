# No/Low config terminal IDE - designed for me!

# Info
This is a very early version of the editor, currently LSP is supported and tested for rust (rust-analyzer) and partially for python with jedi-language-server. Thouse are set as defaults. You will need to supply the LSP servers on your own. And configure them in the .config file this could be done in the integrated terminal (CTRL + ~) with command %i load config.

In the integrated terminal you can also run %i help to get some besic idea what can be configured and most importantly the key map (very similar to VS code).

More documentation will not come very soon - current focus is developing basic features, although I would say the editor is quite usable.

# Tested platform:
- Linux Fedora derivate

## TODO!
- !! more tests - till that point the goal has been to make the editor self-developing, so bugs can be easily found and structure crystalizes
- multi-cursor support
- runner history
- runner autocomplete on dirs
- make info on autocomplete easier to read
- (backlog) semantic token styles for different lsp langs


## Target langs:
* RUST
* Python
* JS/TS
* HTML/JSON/TOML/{YAML/YML}