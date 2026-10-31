# Install

`smb` ships as a single native binary. Pick the channel that matches your toolchain.

## Homebrew (macOS and Linux)

```sh
brew tap smbcloudXYZ/tap && brew trust --tap smbcloudXYZ/tap
brew install cli
```

See [Homebrew](./cli-install-homebrew.md).

## npm

```sh
npm install -g @smbcloud/cli
```

See [npm](./cli-install-npm.md).

## Cargo

```sh
export CLI_CLIENT_SECRET=<secret>
cargo install smbcloud-cli
```

Cargo builds from source and needs `CLI_CLIENT_SECRET` at compile time. See [Native (Cargo)](./cli-install.md). The other channels ship pre-built binaries and need no secret.

## .NET tool

```sh
dotnet tool install --global SmbCloud.Cli
```

## pip

```sh
pip install smbcloud-cli
```

See [PyPI](./cli-install-pypi.md).

## Shell (macOS and Linux)

```sh
curl -fsSL https://raw.githubusercontent.com/smbcloudXYZ/smbcloud-cli/main/install-unix.sh | sh
```

## PowerShell (Windows)

```powershell
irm https://raw.githubusercontent.com/smbcloudXYZ/smbcloud-cli/main/install-windows.sh | iex
```

## Pre-built binaries

Download a binary from the [Releases](https://github.com/smbcloudXYZ/smbcloud-cli/releases) page.

## Verify

```sh
smb --version
```
