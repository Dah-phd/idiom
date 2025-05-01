# Low config terminal IDE - designed for me

## Info
Code editor I develop for myself, both as learning experience and fun side project.

Main goal is to work with code, as a result the current implementation do not use rope under the hood - this means performance when working with long lines might not be optimal, and storage in text document will be opinionated.

**The project is currently in development - so if you want to try it do it with caution.**
This is a very early version of the editor, currently LSP is supported and tested for rust (rust-analyzer) and partially for python with jedi-language-server. Thouse are set as defaults. You will need to supply the LSP servers on your own. And configure them in the .config file this could be done in the integrated terminal (CTRL + ~) with command %i load config.

The package can be installed with cargo:
```shell
cargo install idiom
```

**Currently best language for usage is Rust. You will need to install the LSP manually.**
```shell
rustup component add rust-analyzer
```
Python should work for the most part with jedi, but the interaction is not well optimized. I used the editor for part of its development and optimized the LSP interactions.

### Screen shots
![](/non_dev/screen1.png)

## Tested platform
- Linux Fedora derivate (Nobara / Mint)
- Linux Mint

## TODO
- check can you intergrate ripgrep search
- git integration
- fix tracking edgecase where file is changed in by other app in select
- write tests
- lsp server cold start, maybe? "jedi-language server" starts slow

## Initial target langs
1. RUST
2. Python
3. JS/TS
4. HTML/JSON/TOML/{YAML/YML}
