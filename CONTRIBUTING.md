# Contributing to smbcloud-cli

Thank you for considering contributing to smbcloud-cli! This document outlines the process for contributing to this project.

## Code of Conduct

By participating in this project, you agree to abide by our Code of Conduct. Please be respectful and considerate of others.

## Getting Started

### Prerequisites

- Git
- Node.js (recommended version: 14.x or later)
- npm or yarn

### Setting up your development environment

1. Fork the repository on GitHub
2. Clone your fork locally:
   ```
   git clone https://github.com/your-username/smbcloud-cli.git
   cd smbcloud-cli
   ```
3. Install dependencies:
   ```
   npm install
   ```
   or if you use yarn:
   ```
   yarn install
   ```
4. Create a branch for your work:
   ```
   git checkout -b feature/your-feature-name
   ```

## Development Workflow

1. Make your changes in your feature branch
2. Add tests for your changes
3. Ensure all tests pass with `npm test`
4. Ensure your code follows the style guidelines with `npm run lint`
5. Commit your changes (see Commit Message Guidelines below)
6. Push to your fork and submit a pull request

### Commit Message Guidelines

We follow conventional commit messages. Each commit message should consist of:

- **type**: What kind of change is this?
  - `feat`: A new feature
  - `fix`: A bug fix
  - `docs`: Documentation changes
  - `style`: Code style changes (formatting, missing semicolons, etc)
  - `refactor`: Code changes that neither fix bugs nor add features
  - `perf`: Performance improvements
  - `test`: Adding or updating tests
  - `chore`: Changes to build process or auxiliary tools

Example:
```
feat: add command to list SMB shares
```

## Pull Request Process

1. Update the README.md or documentation with details of your changes, if appropriate
2. Ensure your code passes all tests and linting
3. The PR should address a single concern and be reasonably sized
4. A project maintainer will review your changes and may request modifications
5. Once approved, your PR will be merged

## Testing

- All new features should include appropriate tests
- Run the existing test suite with `npm test` to ensure your changes don't break existing functionality
- For testing CLI commands, consider using a tool like [commander](https://github.com/tj/commander.js/) or [oclif](https://oclif.io/)

## Style Guide

- We use ESLint to enforce code style
- Run `npm run lint` to check your code
- Ideally, configure your editor to show ESLint errors/warnings

## Issue Reporting

- Use the GitHub issue tracker to report bugs or suggest features
- For bugs, please include:
  - A clear description of the issue
  - Steps to reproduce
  - Expected behavior
  - Actual behavior
  - Your environment details (OS, Node.js version, etc.)
- For feature requests, please include:
  - A clear description of the feature
  - Any relevant examples or use cases

## Documentation

- Keep documentation up to date as you make changes
- Document all public APIs and CLI commands
- Update the README.md if adding new features or changing existing behavior

## Releasing

Project maintainers will handle the release process. Generally, we follow semantic versioning (MAJOR.MINOR.PATCH).

## Questions?

If you have any questions about contributing, please reach out to the project maintainers through GitHub issues.

Thank you for your contributions!
