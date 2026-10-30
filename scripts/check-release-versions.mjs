import { existsSync, readFileSync, readdirSync } from "node:fs";
import { resolve } from "node:path";

const repoRoot = resolve(import.meta.dirname, "..");
const expectedVersion = readFileSync(resolve(repoRoot, "crates/cli/Cargo.toml"), "utf8")
  .match(/^version\s*=\s*"([^"]+)"/m)?.[1];

if (!expectedVersion) {
  throw new Error("Unable to determine the release version from crates/cli/Cargo.toml");
}

const mismatches = [];

function read(path) {
  return readFileSync(resolve(repoRoot, path), "utf8");
}

function check(path, actualVersion) {
  if (actualVersion !== expectedVersion) {
    mismatches.push(`${path}: ${actualVersion ?? "missing"} (expected ${expectedVersion})`);
  }
}

function cargoPackageVersion(path) {
  return read(path).match(/^\[package\][\s\S]*?^version\s*=\s*"([^"]+)"/m)?.[1];
}

for (const directory of ["crates", "sdk/gems"]) {
  const directoryPath = resolve(repoRoot, directory);
  for (const entry of readdirSync(directoryPath, { withFileTypes: true })) {
    if (!entry.isDirectory()) {
      continue;
    }

    const candidates = [
      `${directory}/${entry.name}/Cargo.toml`,
      ...readdirSync(resolve(directoryPath, entry.name), { withFileTypes: true })
        .filter((nestedEntry) => nestedEntry.isDirectory())
        .map((nestedEntry) => `${directory}/${entry.name}/${nestedEntry.name}/Cargo.toml`),
    ];

    for (const candidate of candidates) {
      if (existsSync(resolve(repoRoot, candidate))) {
        const version = cargoPackageVersion(candidate);
        if (version) {
          check(candidate, version);
        }
      }
    }
  }
}

for (const match of read("Cargo.toml").matchAll(
  /^([\w-]+)\s*=\s*\{\s*version\s*=\s*"([^"]+)"\s*,\s*path\s*=\s*"crates\//gm,
)) {
  check(`Cargo.toml workspace dependency ${match[1]}`, match[2]);
}

for (const path of [
  "sdk/gems/auth/lib/auth/version.rb",
  "sdk/gems/email/lib/email/version.rb",
  "sdk/gems/model/lib/model/version.rb",
]) {
  check(path, read(path).match(/VERSION\s*=\s*['"]([^'"]+)['"]/)?.[1]);
}

// The Ruby gems link the smbcloud-* crates from crates.io. Their requirement
// must be the release's major.minor, or a gem can silently wrap an older crate.
const [expectedMajor, expectedMinor] = expectedVersion.split(".");
const expectedRequirement = `${expectedMajor}.${expectedMinor}`;
for (const path of [
  "sdk/gems/auth/ext/auth/Cargo.toml",
  "sdk/gems/email/ext/email/Cargo.toml",
  "sdk/gems/model/ext/model/Cargo.toml",
]) {
  for (const match of read(path).matchAll(/^(smbcloud-[\w-]+)\s*=\s*\{\s*version\s*=\s*"([^"]+)"/gm)) {
    if (match[2] !== expectedRequirement) {
      mismatches.push(`${path} dependency ${match[1]}: ${match[2]} (expected ${expectedRequirement})`);
    }
  }
}

const npmPackage = JSON.parse(read("packaging/npm/smbcloud-cli/package.json"));
check("packaging/npm/smbcloud-cli/package.json", npmPackage.version);

const sdkNpmPackage = JSON.parse(read("sdk/npm/smbcloud-auth/package.json"));
check("sdk/npm/smbcloud-auth/package.json", sdkNpmPackage.version);

const npmLock = JSON.parse(read("packaging/npm/smbcloud-cli/package-lock.json"));
check("packaging/npm/smbcloud-cli/package-lock.json", npmLock.version);
check("packaging/npm/smbcloud-cli/package-lock.json packages root", npmLock.packages?.[""]?.version);

const npmXcrsPackage = JSON.parse(read("packaging/npm/xcrs/package.json"));
check("packaging/npm/xcrs/package.json", npmXcrsPackage.version);

const npmXcrsLock = JSON.parse(read("packaging/npm/xcrs/package-lock.json"));
check("packaging/npm/xcrs/package-lock.json", npmXcrsLock.version);
check("packaging/npm/xcrs/package-lock.json packages root", npmXcrsLock.packages?.[""]?.version);

for (const path of ["server.json", "server-xcrs.json"]) {
  const server = JSON.parse(read(path));
  check(path, server.version);
  for (const [index, packageEntry] of (server.packages ?? []).entries()) {
    check(`${path} package ${index + 1}`, packageEntry.version);
  }
}

check(
  "packaging/nuget/smbcloud-cli/SmbCloud.Cli.csproj",
  read("packaging/nuget/smbcloud-cli/SmbCloud.Cli.csproj").match(/<PackageVersion>([^<]+)<\/PackageVersion>/)?.[1],
);
check(
  "packaging/nuget/xcrs/Xcrs.csproj",
  read("packaging/nuget/xcrs/Xcrs.csproj").match(/<PackageVersion>([^<]+)<\/PackageVersion>/)?.[1],
);
check(
  "sdk/gems/email/Gemfile.lock",
  read("sdk/gems/email/Gemfile.lock").match(/smbcloud-email \(([^)]+)\)/)?.[1],
);

if (mismatches.length > 0) {
  console.error(`Release version mismatch detected for ${expectedVersion}:`);
  for (const mismatch of mismatches) {
    console.error(`- ${mismatch}`);
  }
  process.exit(1);
}

console.log(`All release artifacts use version ${expectedVersion}.`);
