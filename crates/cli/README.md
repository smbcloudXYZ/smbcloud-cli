<!-- LOGO -->
<h1>
<p align="center">
  <img src="https://avatars.githubusercontent.com/u/89791739?s=200&v=4" alt="smbCloud Logo" width="128">
  <br>smbCloud CLI
</h1>
  <p align="center">
    Deploy to the cloud in one command.
    <br />
    <br />
    <a href="https://smbcloud.xyz/">Website</a>
    ·
    <a href="https://smbcloud.xyz/posts">Documentation</a>
    ·
    <a href="https://github.com/smbcloudXYZ/smbcloud-cli/releases">Releases</a>
  </p>
  <p align="center">
    <a href="https://crates.io/crates/smbcloud-cli"><img alt="Crates.io" src="https://img.shields.io/crates/v/smbcloud-cli"></a>
    <a href="https://github.com/smbcloudXYZ/smbcloud-cli/blob/main/LICENSE"><img alt="License" src="https://img.shields.io/github/license/smbcloudXYZ/smbcloud-cli"></a>
  </p>
</p>

## Installation

You can install the CLI a few different ways.

### With Cargo

```bash
cargo install smbcloud-cli
```

### Homebrew (MacOS/Linux)

```bash
brew tap smbcloudXYZ/tap
brew install cli
```

### With NPM

```bash
npm install -g @smbcloud/cli
```

### With PyPI

```bash
pip install smbcloud-cli
```

## Update

To update the CLI, run the same install command again.

## Uninstall

```bash
# With cargo
cargo uninstall smbcloud-cli

# With npm
npm uninstall -g @smbcloud/cli

# With Homebrew
brew uninstall cli
brew untap smbcloudXYZ/tap

# With pip
pip uninstall smbcloud-cli
```

## Usage

```bash
smb --help
```

## Contribution

- Set up your Rust tooling.
- Clone the repo.
- Add the required environment variables to `.env.local`.
- Run `cargo run`.

## Credits

This repo draws inspiration from [Sugar](https://github.com/metaplex-foundation/sugar).

It also borrows ideas from [the 12 factor CLI app](https://medium.com/@jdxcode/12-factor-cli-apps-dd3c227a0e46).

> Explore more on the [smbCloud Services](https://smbcloud.xyz/services) page.

## License

[Apache-2.0](https://github.com/smbcloudXYZ/smbcloud-cli/blob/main/LICENSE)

## Copyright

© 2026 [smbCloud](https://smbcloud.xyz) (Splitfire AB).
