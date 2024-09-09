# Low config terminal IDE - designed for me

## Info
Code editor I develop for myself, both as learning experience and fun side project.

Main goal is to work with code, as a result the current implementation do not use rope under the hood - this means performance when working with long lines might not be optimal, and storage in text document will be opinionated.

**The project is currently in development - so if you want to try it do it with caution.**
This is a very early version of the editor, currently LSP is supported and tested for rust (rust-analyzer) and partially for python with jedi-language-server. Thouse are set as defaults. You will need to supply the LSP servers on your own. And configure them in the .config file this could be done in the integrated terminal (CTRL + ~) with command %i load config.

### Screen shots
![](/non_dev/screen1.png)

## Tested platform
- Linux Fedora derivate (Nobara)
- Linux Mint

## TODO
- write tests
- write todos

## Initial target langs

- RUST
- Python
- JS/TS
- HTML/JSON/TOML/{YAML/YML}
