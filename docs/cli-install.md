# Native Installation

Install directly from [crates.io](https://crates.io/crates/smbcloud-cli) using Cargo.

## Prerequisites

- [Rust](https://rustup.rs) (stable toolchain)
- `CLI_CLIENT_SECRET` — required at compile time

The `CLI_CLIENT_SECRET` value is baked into the binary at compile time. Contact [hej@setoelkahfi.se](mailto:hej@setoelkahfi.se) or check your smbCloud account dashboard to obtain it.

## Install

Export the secret, then install:

```bash
export CLI_CLIENT_SECRET=your_secret_here
cargo install smbcloud-cli
```

## Verify

```bash
smb --version
```

## Upgrade

```bash
export CLI_CLIENT_SECRET=your_secret_here
cargo install smbcloud-cli --force
```

## Uninstall

```bash
cargo uninstall smbcloud-cli
```

## Troubleshooting

**`error: proc-macro derive panicked … Error loading .env file`**

This error appears on older versions of `smbcloud-cli` (before `0.3.37`). Upgrade to the latest version — it reads `CLI_CLIENT_SECRET` directly from the environment at compile time and no longer requires a `.env` file.

**`error[E0463]: can't find crate for std`**

Your Rust toolchain is missing the target. Run:

```bash
rustup target add <target>
```
