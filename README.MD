# smbcloud-cli

**smbcloud-cli** is a robust, open-source command-line interface (CLI) designed for seamless integration with [smbcloud](https://smbcloud.xyz/)—the modern Next.js deployment platform for businesses and developers. Deploy your Next.js apps with the simplicity of a "git push", and enjoy a developer experience on par with Vercel, Cloudflare, and AWS Amplify.

## Key Features

- **Effortless Next.js Deployments:** Instantly deploy your Next.js projects to smbcloud with a single command or via "git push to deploy".
- **Vercel/Cloudflare/AWS Amplify Alternative:** Enterprise-grade platform for modern frontend and full-stack applications.
- **Cross-Platform:** Works on MacOS, Linux, and Windows.
- **Open Source:** MIT-licensed and community-driven.
- **Developer Friendly:** Easy to install, extend, and contribute.

## Installation

Choose the installation method that best fits your workflow:

### With Cargo

```bash
cargo install smbcloud-cli
```

### Homebrew (MacOS/Linux)

```bash
brew tap smbcloudXYZ/tap
brew install cli
```

### With NPM/Pnpm/Bun/Deno

```bash
npm i -g @smbcloud/cli
```

## Updating

Stay up-to-date with the latest features and security patches by rerunning your installation command.

## Uninstallation

Remove smbcloud-cli using your preferred package manager:

```bash
# With Cargo
cargo uninstall smbcloud-cli

# With NPM
npm uninstall -g @smbcloud/cli

# With Homebrew
brew uninstall smbcloudXYZ/tap/cli
brew untap smbcloudXYZ/tap 
```

## Getting Started

Explore available commands and options:

```bash
smb --help
```

## Contributing

We welcome contributions from the community and businesses alike. To get started:

- Ensure you have the latest Rust toolchain installed.
- Clone this repository.
- Set up your environment variables in `.env.local`.
- Run the CLI locally with `cargo run`.

For detailed contribution guidelines, please refer to [CONTRIBUTING.md](CONTRIBUTING.md) (coming soon).

## Why smbcloud-cli?

- **Next.js First:** Purpose-built for Next.js, with zero-config deployments and instant rollbacks.
- **Git Push to Deploy:** Seamless CI/CD—just push to your git repository and your app is live.
- **Enterprise-Ready:** Built with reliability and scalability in mind.
- **Open Collaboration:** Inspired by industry best practices and the open-source community.
- **Modern CLI Principles:** Follows [12-factor CLI app](https://medium.com/@jdxcode/12-factor-cli-apps-dd3c227a0e46) methodology for maintainability and user experience.

## Credits

- Inspired by [Sugar](https://github.com/metaplex-foundation/sugar).
- NPM packaging guidance by [orhun.dev](https://blog.orhun.dev/packaging-rust-for-npm/).

## License

MIT License. See [LICENSE](LICENSE) for details.

---

**smbcloud-cli**: The professional's choice for secure, efficient, and open Next.js cloud deployments—experience the power of "git push to deploy".
