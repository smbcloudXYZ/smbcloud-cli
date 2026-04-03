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
    <a href="https://www.smbcloud.xyz/">Website</a>
    ·
    <a href="./docs">Documentation</a>
    ·
    <a href="CONTRIBUTING.md">Contributing</a>
    ·
    <a href="https://github.com/smbcloudXYZ/smbcloud-cli/releases">Releases</a>
  </p>
  <p align="center">
    <a href="https://crates.io/crates/smbcloud-cli"><img alt="crates.io" src="https://img.shields.io/crates/v/smbcloud-cli"></a>
    <a href="https://www.npmjs.com/package/@smbcloud/cli"><img alt="npm" src="https://img.shields.io/npm/v/@smbcloud/cli"></a>
    <a href="https://pypi.org/project/smbcloud-cli/"><img alt="PyPI" src="https://img.shields.io/pypi/v/smbcloud-cli"></a>
    <a href="https://github.com/smbcloudXYZ/smbcloud-cli/blob/main/LICENSE"><img alt="License" src="https://img.shields.io/github/license/smbcloudXYZ/smbcloud-cli"></a>
  </p>
</p>

## About

**`smb`** is the command-line interface for [smbCloud](https://www.smbcloud.xyz/) — the modern cloud deployment platform. Ship your Rust, Node.js, Ruby, or Swift app with a single command.

## Install

### Homebrew (macOS)

```sh
brew tap smbcloudXYZ/homebrew-tap
brew install cli
```

### npm

```sh
npm install -g @smbcloud/cli
```

### pip

```sh
pip install smbcloud-cli
```

### Cargo

```sh
cargo install smbcloud-cli
```

### Shell (macOS / Linux)

```sh
curl -fsSL https://raw.githubusercontent.com/smbcloudXYZ/smbcloud-cli/main/install-unix.sh | sh
```

### PowerShell (Windows)

```powershell
irm https://raw.githubusercontent.com/smbcloudXYZ/smbcloud-cli/main/install-windows.sh | iex
```

Or grab a pre-built binary from the [Releases](https://github.com/smbcloudXYZ/smbcloud-cli/releases) page.

## Quick Start

```sh
smb login
smb init
smb deploy
```

That's it. Your app is live.

## Documentation

See the [`docs/`](./docs) directory for guides on authentication, project configuration, and deployment workflows.

## Contributing

Read [CONTRIBUTING.md](CONTRIBUTING.md) to get started. All contributions are welcome.

## License

[Apache-2.0](LICENSE)
