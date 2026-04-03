# Installation

Deploying to the cloud should be absolutely frictionless. We provide pre-built binaries and seamless installation across all major platforms. Choose the method that best fits your workflow.

### Homebrew (macOS & Linux)

The fastest way to get started on Apple Silicon, Intel Macs, and Linux. We maintain an official tap with highly optimized, pre-built binaries so you can install `smb` in seconds.

```bash
brew tap smbcloudXYZ/tap
brew install cli
```

### Node Package Managers (npm / pnpm / bun / yarn)

Perfect for JavaScript and TypeScript developers. The CLI is distributed as a lightweight, native binary wrapper.

```bash
npm install -g @smbcloud/cli
```

## Updating

To get the latest features, speed improvements, and security patches:

```bash
# Homebrew
brew upgrade cli

# NPM
npm update -g @smbcloud/cli


```

## Uninstallation

Remove the CLI using your preferred package manager:

```bash
# Homebrew
brew uninstall cli
brew untap smbcloudXYZ/tap

# NPM
npm uninstall -g @smbcloud/cli


```

## Getting Started

Explore the available commands and deploy your first app:

```bash
smb --help
```
