#!/usr/bin/env node

const fs = require("fs");
const path = require("path");

const [outputPath, version, product = "smb"] = process.argv.slice(2);

if (!outputPath || !version) {
    throw new Error("Usage: render-main-package.cjs <output-path> <version> [product]");
}

// The five platform binary packages the wrapper resolves at runtime. Kept in
// lockstep with the release build matrix and the `optionalDependencies` below.
const platformSuffixes = [
    "darwin-arm64",
    "darwin-x64",
    "linux-x64",
    "windows-arm64",
    "windows-x64",
];

const products = {
    smb: {
        name: "@smbcloud/cli",
        // Ownership proof for the MCP Registry: must match `name` in ../../server.json.
        mcpName: "io.github.smbcloudXYZ/smbcloud-cli",
        binName: "smb",
        platformPrefix: "cli",
        description: "A CLI for accessing the smbCloud platform.",
        keywords: [
            "smbcloud",
            "cli",
            "smb",
            "pndk",
            "smbcloud-cli",
            "cloud",
            "smbcloud",
            "developer",
            "tools",
            "web3",
            "web3js",
            "web3.js",
            "web3js",
            "bitcoin",
            "btc",
            "ethereum",
            "solana",
            "eth",
            "sol",
            "blockchain",
            "smart-contracts",
            "dapps",
            "dapp",
            "dapp-tools",
            "dapp-tool",
            "dapp-toolkit",
            "dapp-toolkit-cli",
        ],
    },
    xcrs: {
        name: "@smbcloud/xcrs",
        // Ownership proof for the MCP Registry: must match `name` in ../../server-xcrs.json.
        mcpName: "io.github.smbcloudXYZ/xcrs",
        binName: "xcrs",
        platformPrefix: "xcrs",
        description: "Cross-platform mobile and TV app automation over MCP.",
        keywords: [
            "xcrs",
            "mcp",
            "model-context-protocol",
            "mobile",
            "ios",
            "android",
            "tv",
            "apple-tv",
            "android-tv",
            "automation",
            "app-testing",
            "device",
            "simulator",
            "adb",
            "cli",
        ],
    },
};

const selected = products[product];

if (!selected) {
    throw new Error(
        `Unknown product "${product}". Expected one of: ${Object.keys(products).join(", ")}.`
    );
}

const optionalDependencies = {};
for (const suffix of platformSuffixes) {
    optionalDependencies[`@smbcloud/${selected.platformPrefix}-${suffix}`] = version;
}

const packageJson = {
    name: selected.name,
    version,
    mcpName: selected.mcpName,
    keywords: selected.keywords,
    bin: {
        [selected.binName]: "lib/index.js",
    },
    files: ["lib", "README.md"],
    description: selected.description,
    license: "Apache-2.0",
    repository: {
        type: "git",
        url: "git+https://github.com/smbcloudXYZ/smbcloud-cli.git",
    },
    scripts: {
        typecheck: "tsc --noEmit",
        lint: "eslint .",
        "lint:fix": "eslint . --fix",
        build: "tsc",
        dev: "npm run build && node lib/index.js",
    },
    devDependencies: {
        "@types/node": "^18.15.11",
        "@typescript-eslint/eslint-plugin": "^5.48.0",
        "@typescript-eslint/parser": "^5.48.0",
        eslint: "^8.31.0",
        typescript: "^5.0.0",
    },
    optionalDependencies,
};

fs.writeFileSync(path.resolve(outputPath), `${JSON.stringify(packageJson, null, 2)}
`);
