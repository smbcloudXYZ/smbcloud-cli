#!/usr/bin/env node

const fs = require("fs");
const path = require("path");

const [packageName, version, operatingSystem, architecture] = process.argv.slice(2);

if (!packageName || !version || !operatingSystem || !architecture) {
    throw new Error(
        "Usage: render-platform-package.cjs <package-name> <version> <os> <arch>"
    );
}

const packageDirectory = path.resolve(__dirname, "..", packageName);

fs.mkdirSync(packageDirectory, { recursive: true });

const packageJson = {
    name: `@smbcloud/${packageName}`,
    version,
    description: "Platform binary for the smbCloud CLI.",
    license: "Apache-2.0",
    repository: {
        type: "git",
        url: "git+https://github.com/smbcloudXYZ/smbcloud-cli.git",
    },
    keywords: [
        packageName,
        "cli",
        "binary",
        "node",
        "nodejs",
        "npm",
        "package",
        "smbcloud",
    ],
    os: [operatingSystem],
    cpu: [architecture],
    files: ["bin"],
};

const readme = `# ${packageName}

Not meant to be used directly. Please see [@smbcloud/cli](https://www.npmjs.com/package/@smbcloud/cli).
`;

fs.writeFileSync(
    path.join(packageDirectory, "package.json"),
    `${JSON.stringify(packageJson, null, 2)}
`
);
fs.writeFileSync(path.join(packageDirectory, "README.md"), readme);
