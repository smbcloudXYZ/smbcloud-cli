import { cpSync, existsSync, mkdirSync, readFileSync, rmSync } from "node:fs";
import { resolve } from "node:path";
import { execFileSync } from "node:child_process";

const packageDir = resolve(import.meta.dirname);
const repoRoot = resolve(packageDir, "../../..");
const crateDir = resolve(repoRoot, "crates/smbcloud-auth-wasm");
const stageDir = resolve(repoRoot, "target/npm/sdk-auth");
const distDir = resolve(packageDir, "dist");
const npmPackageJsonPath = resolve(packageDir, "package.json");
const cargoTomlPath = resolve(crateDir, "Cargo.toml");

const stageFiles = [
    "smbcloud_auth_wasm.js",
    "smbcloud_auth_wasm.d.ts",
    "smbcloud_auth_wasm_bg.wasm",
    "smbcloud_auth_wasm_bg.wasm.d.ts",
];

const npmPackage = JSON.parse(readFileSync(npmPackageJsonPath, "utf8"));
const cargoToml = readFileSync(cargoTomlPath, "utf8");
const cargoVersionMatch = cargoToml.match(/^version\s*=\s*"([^"]+)"/m);

if (!cargoVersionMatch) {
    throw new Error(`Unable to find version in ${cargoTomlPath}`);
}

const crateVersion = cargoVersionMatch[1];
const npmVersion = npmPackage.version;

if (crateVersion !== npmVersion) {
    throw new Error(
        [
            "Version mismatch for @smbcloud/sdk-auth.",
            `npm package version: ${npmVersion}`,
            `Rust crate version: ${crateVersion}`,
            "Update sdk/npm/smbcloud-auth/package.json or crates/smbcloud-auth-wasm/Cargo.toml so they match before publishing.",
        ].join("\n"),
    );
}

rmSync(stageDir, { recursive: true, force: true });
rmSync(distDir, { recursive: true, force: true });
mkdirSync(stageDir, { recursive: true });
mkdirSync(distDir, { recursive: true });

execFileSync(
    "wasm-pack",
    ["build", crateDir, "--target", "web", "--out-dir", stageDir],
    { cwd: repoRoot, stdio: "inherit" },
);

for (const file of stageFiles) {
    const source = resolve(stageDir, file);
    const destination = resolve(distDir, file);

    if (!existsSync(source)) {
        throw new Error(`Missing generated artifact: ${source}`);
    }

    cpSync(source, destination);
}

console.log("Prepared @smbcloud/sdk-auth package in", packageDir);
