# Installation

## From crates.io

```bash
cargo install laurus-cli
```

This installs the `laurus` binary to `~/.cargo/bin/`.

## From source

```bash
git clone https://github.com/mosuka/laurus.git
cd laurus
cargo install --path laurus-cli
```

## Verify

```bash
laurus --version
```

## Shell Completion

Generate completion scripts for your shell:

```bash
# Bash
laurus --help

# The CLI uses clap, so shell completions can be generated
# with clap_complete if needed in a future release.
```
