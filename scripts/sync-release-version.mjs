import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";

const repoRoot = resolve(import.meta.dirname, "..");
const cliCargoTomlPath = resolve(repoRoot, "crates/cli/Cargo.toml");
const npmCliPackageJsonPath = resolve(repoRoot, "npm/smbcloud-cli/package.json");
const npmCliPackageLockPath = resolve(repoRoot, "npm/smbcloud-cli/package-lock.json");
const nugetCliProjectPath = resolve(repoRoot, "nuget/smbcloud-cli/SmbCloud.Cli.csproj");
const sdkNpmPackageJsonPath = resolve(repoRoot, "sdk/npm/smbcloud-auth/package.json");
const authGemVersionPath = resolve(repoRoot, "sdk/gems/auth/lib/auth/version.rb");
const authGemCargoTomlPath = resolve(repoRoot, "sdk/gems/auth/ext/auth/Cargo.toml");
const modelGemVersionPath = resolve(repoRoot, "sdk/gems/model/lib/model/version.rb");
const modelGemCargoTomlPath = resolve(repoRoot, "sdk/gems/model/ext/model/Cargo.toml");
const emailGemVersionPath = resolve(repoRoot, "sdk/gems/email/lib/email/version.rb");
const emailGemCargoTomlPath = resolve(repoRoot, "sdk/gems/email/ext/email/Cargo.toml");
const emailGemLockPath = resolve(repoRoot, "sdk/gems/email/Gemfile.lock");
const mcpServerJsonPath = resolve(repoRoot, "server.json");
const xcrsMcpServerJsonPath = resolve(repoRoot, "server-xcrs.json");

function readText(path) {
  return readFileSync(path, "utf8");
}

function writeText(path, content) {
  writeFileSync(path, content, "utf8");
}

function readCargoPackageVersion(path) {
  const match = readText(path).match(/^version\s*=\s*"([^"]+)"/m);

  if (!match) {
    throw new Error(`Unable to find version in ${path}`);
  }

  return match[1];
}

function replaceOrThrow(content, pattern, replacement, description) {
  if (!pattern.test(content)) {
    throw new Error(`Unable to update ${description}`);
  }

  return content.replace(pattern, replacement);
}

function updateFile(path, updater) {
  const current = readText(path);
  const next = updater(current);

  if (next !== current) {
    writeText(path, next);
    return true;
  }

  return false;
}

const releaseVersion = readCargoPackageVersion(cliCargoTomlPath);
const [major = "0", minor = "0"] = releaseVersion.split(".");
const rubySdkRequirement = `${major}.${minor}`;
const updatedPaths = [];

if (
  updateFile(npmCliPackageJsonPath, (content) => {
    const parsed = JSON.parse(content);
    parsed.version = releaseVersion;

    for (const packageName of Object.keys(parsed.optionalDependencies ?? {})) {
      parsed.optionalDependencies[packageName] = releaseVersion;
    }

    return `${JSON.stringify(parsed, null, 2)}\n`;
  })
) {
  updatedPaths.push(npmCliPackageJsonPath);
}

if (
  updateFile(npmCliPackageLockPath, (content) => {
    const parsed = JSON.parse(content);
    parsed.version = releaseVersion;
    parsed.packages[""].version = releaseVersion;

    for (const packageName of Object.keys(parsed.packages[""].optionalDependencies ?? {})) {
      parsed.packages[""].optionalDependencies[packageName] = releaseVersion;
    }

    return `${JSON.stringify(parsed, null, 2)}\n`;
  })
) {
  updatedPaths.push(npmCliPackageLockPath);
}

if (
  updateFile(nugetCliProjectPath, (content) =>
    replaceOrThrow(
      content,
      /<PackageVersion>[^<]+<\/PackageVersion>/,
      `<PackageVersion>${releaseVersion}</PackageVersion>`,
      `${nugetCliProjectPath} package version`,
    ),
  )
) {
  updatedPaths.push(nugetCliProjectPath);
}

if (
  updateFile(sdkNpmPackageJsonPath, (content) => {
    const parsed = JSON.parse(content);
    parsed.version = releaseVersion;
    return `${JSON.stringify(parsed, null, 2)}\n`;
  })
) {
  updatedPaths.push(sdkNpmPackageJsonPath);
}

if (
  updateFile(authGemVersionPath, (content) =>
    replaceOrThrow(
      content,
      /VERSION\s*=\s*'[^']+'/,
      `VERSION = '${releaseVersion}'`,
      `${authGemVersionPath} VERSION constant`,
    ),
  )
) {
  updatedPaths.push(authGemVersionPath);
}

if (
  updateFile(authGemCargoTomlPath, (content) => {
    let next = replaceOrThrow(
      content,
      /^version\s*=\s*"[^"]+"/m,
      `version = "${releaseVersion}"`,
      `${authGemCargoTomlPath} package version`,
    );

    for (const crateName of [
      "smbcloud-auth-sdk",
      "smbcloud-model",
      "smbcloud-network",
    ]) {
      next = replaceOrThrow(
        next,
        new RegExp(`(${crateName}\\s*=\\s*\\{\\s*version\\s*=\\s*")([^"]+)("\\s*\\})`),
        `$1${rubySdkRequirement}$3`,
        `${authGemCargoTomlPath} dependency ${crateName}`,
      );
    }

    return next;
  })
) {
  updatedPaths.push(authGemCargoTomlPath);
}

if (
  updateFile(modelGemVersionPath, (content) =>
    replaceOrThrow(
      content,
      /VERSION\s*=\s*'[^']+'/,
      `VERSION = '${releaseVersion}'`,
      `${modelGemVersionPath} VERSION constant`,
    ),
  )
) {
  updatedPaths.push(modelGemVersionPath);
}

if (
  updateFile(modelGemCargoTomlPath, (content) =>
    replaceOrThrow(
      content,
      /^version\s*=\s*"[^"]+"/m,
      `version = "${releaseVersion}"`,
      `${modelGemCargoTomlPath} package version`,
    ),
  )
) {
  updatedPaths.push(modelGemCargoTomlPath);
}

if (
  updateFile(emailGemVersionPath, (content) =>
    replaceOrThrow(
      content,
      /VERSION\s*=\s*'[^']+'/,
      `VERSION = '${releaseVersion}'`,
      `${emailGemVersionPath} VERSION constant`,
    ),
  )
) {
  updatedPaths.push(emailGemVersionPath);
}

if (
  updateFile(emailGemCargoTomlPath, (content) =>
    replaceOrThrow(
      content,
      /^version\s*=\s*"[^"]+"/m,
      `version = "${releaseVersion}"`,
      `${emailGemCargoTomlPath} package version`,
    ),
  )
) {
  updatedPaths.push(emailGemCargoTomlPath);
}

if (
  updateFile(emailGemLockPath, (content) =>
    replaceOrThrow(
      content,
      /(smbcloud-email \()[^)]+(\))/,
      `$1${releaseVersion}$2`,
      `${emailGemLockPath} gem version`,
    ),
  )
) {
  updatedPaths.push(emailGemLockPath);
}

if (
  updateFile(mcpServerJsonPath, (content) => {
    const parsed = JSON.parse(content);
    parsed.version = releaseVersion;

    for (const pkg of parsed.packages ?? []) {
      pkg.version = releaseVersion;
    }

    return `${JSON.stringify(parsed, null, 2)}\n`;
  })
) {
  updatedPaths.push(mcpServerJsonPath);
}

if (
  updateFile(xcrsMcpServerJsonPath, (content) => {
    const parsed = JSON.parse(content);
    parsed.version = releaseVersion;

    for (const pkg of parsed.packages ?? []) {
      pkg.version = releaseVersion;
    }

    return `${JSON.stringify(parsed, null, 2)}\n`;
  })
) {
  updatedPaths.push(xcrsMcpServerJsonPath);
}

console.log(`Release version: ${releaseVersion}`);

if (updatedPaths.length === 0) {
  console.log("Release metadata already in sync.");
} else {
  console.log("Updated release metadata files:");
  for (const path of updatedPaths) {
    console.log(`- ${path}`);
  }
}

console.log(`Ruby SDK dependency requirement: ${rubySdkRequirement}`);
