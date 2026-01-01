# smbcloud-cli

**smbcloud-cli** is a robust, open-source command-line interface (CLI) for [smbcloud](https://smbcloud.xyz/)—the modern cloud deployment platform. Deploy your NodeJs, Ruby, or Swift apps with the simplicity of a `push-to-deploy`.

## Installation

[Cargo](https://doc.rust-lang.org/cargo/) is our main installation method. Setup your [Rust toolchain](https://www.rust-lang.org/tools/install) first.

### With Cargo

```bash
$ cargo install smbcloud-cli
```

### Install Scripts

Probably broken now.

#### Unix-like

```bash
$
$ # Install system-wide
$ curl -s https://raw.githubusercontent.com/reemus-dev/gitnr/main/scripts/install.sh | sudo bash -s
$
$ # Install for current user
$ curl -s https://raw.githubusercontent.com/reemus-dev/gitnr/main/scripts/install.sh | bash -s -- -u
$
$ # Install in specific directory
$ curl -s https://raw.githubusercontent.com/reemus-dev/gitnr/main/scripts/install.sh | bash -s -- -d <dir>
$
```

#### PowerShell

```bash
Set-ExecutionPolicy Unrestricted -Scope Process; iex (iwr "https://raw.githubusercontent.com/reemus-dev/gitnr/main/scripts/install.ps1").Content
```
### Homebrew (MacOS/Linux)

Probably broken now.

```bash
$ brew tap smbcloudXYZ/tap
$ brew install cli
```

### With NPM/Pnpm/Bun/Deno

Probably broken now.

```bash
$ npm i -g @smbcloud/cli
```

## Updating

Simply re-run the install command.

## Uninstallation

Remove smbcloud-cli using your preferred package manager:

```bash
# With Cargo
$ cargo uninstall smbcloud-cli

# With NPM
$ npm uninstall -g @smbcloud/cli

# With Homebrew
$ brew uninstall smbcloudXYZ/tap/cli
$ brew untap smbcloudXYZ/tap
```

## Getting Started

Explore available commands and options:

```bash
smb --help
```

## Credits

- [Sugar](https://github.com/metaplex-foundation/sugar).
- [NPM packaging for Rust](https://blog.orhun.dev/packaging-rust-for-npm/).
- [Install scripts tutorial](https://reemus.dev/tldr/cli-install-script).

## License

MIT License.
