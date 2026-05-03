#!/usr/bin/env node

import { spawnSync } from "child_process";

/**
 * Returns the executable path which is located inside `node_modules`
 * The naming convention is cli-${os}-${arch}
 * If the platform is `win32` or `cygwin`, executable will include a `.exe` extension.
 * @see https://nodejs.org/api/os.html#osarch
 * @see https://nodejs.org/api/os.html#osplatform
 * @example "x/xx/node_modules/cli-darwin-arm64"
 */
function getExePath() {
    const arch = process.arch;
    let os = process.platform as string;
    let extension = "";
    if (["win32", "cygwin"].includes(process.platform)) {
        os = "windows";
        extension = ".exe";
    }

    try {
        // The binary lives in `node_modules`, so `require.resolve` gives us the installed path.
        return require.resolve(
            `@smbcloud/cli-${os}-${arch}/bin/smb${extension}`,
        );
    } catch (e) {
        throw new Error(
            `Couldn't find application binary inside node_modules for ${os}-${arch}`,
        );
    }
}

/**
 * Runs the native CLI with the current process arguments.
 */
function run() {
    const args = process.argv.slice(2);
    const processResult = spawnSync(getExePath(), args, { stdio: "inherit" });
    process.exit(processResult.status ?? 0);
}

run();
