import { e as error, g as getCacheProvider, a as getInput, d as isCacheUpToDate, i as info, C as CacheConfig, c as cleanTargetDir, f as debug, h as cleanRegistry, j as cleanBin, k as cleanGit, r as reportError, l as exec } from './cleanup-C5dkQaqv.js';
import 'os';
import 'crypto';
import 'fs';
import 'path';
import 'http';
import 'https';
import 'net';
import 'tls';
import 'events';
import 'assert';
import 'util';
import 'node:assert';
import 'node:net';
import 'node:http';
import 'node:stream';
import 'node:buffer';
import 'node:util';
import 'node:querystring';
import 'node:events';
import 'node:diagnostics_channel';
import 'node:tls';
import 'node:zlib';
import 'node:perf_hooks';
import 'node:util/types';
import 'node:worker_threads';
import 'node:url';
import 'node:async_hooks';
import 'node:console';
import 'node:dns';
import 'string_decoder';
import 'child_process';
import 'timers';
import 'stream';
import 'fs/promises';

process.on("uncaughtException", (e) => {
    error(e.message);
    if (e.stack) {
        error(e.stack);
    }
});
async function run() {
    const cacheProvider = await getCacheProvider();
    const save = getInput("save-if").toLowerCase() || "true";
    if (!(cacheProvider.cache.isFeatureAvailable() && save === "true")) {
        return;
    }
    try {
        if (isCacheUpToDate()) {
            info(`Cache up-to-date.`);
            return;
        }
        const config = CacheConfig.fromState();
        config.printInfo(cacheProvider);
        info("");
        // TODO: remove this once https://github.com/actions/toolkit/pull/553 lands
        if (process.env["RUNNER_OS"] == "macOS") {
            await macOsWorkaround();
        }
        const workspaceCrates = getInput("cache-workspace-crates").toLowerCase() || "false";
        const allPackages = [];
        for (const workspace of config.workspaces) {
            const packages = await workspace.getPackagesOutsideWorkspaceRoot(config.cmdFormat);
            if (workspaceCrates === "true") {
                const wsMembers = await workspace.getWorkspaceMembers(config.cmdFormat);
                packages.push(...wsMembers);
            }
            allPackages.push(...packages);
            try {
                info(`... Cleaning ${workspace.target} ...`);
                await cleanTargetDir(workspace.target, packages);
            }
            catch (e) {
                debug(`${e.stack}`);
            }
        }
        try {
            const crates = getInput("cache-all-crates").toLowerCase() || "false";
            info(`... Cleaning cargo registry (cache-all-crates: ${crates}) ...`);
            await cleanRegistry(allPackages, crates !== "true");
        }
        catch (e) {
            debug(`${e.stack}`);
        }
        if (config.cacheBin) {
            try {
                info(`... Cleaning cargo/bin ...`);
                await cleanBin(config.cargoBins);
            }
            catch (e) {
                debug(`${e.stack}`);
            }
        }
        try {
            info(`... Cleaning cargo git cache ...`);
            await cleanGit(allPackages);
        }
        catch (e) {
            debug(`${e.stack}`);
        }
        info(`... Saving cache ...`);
        // Pass a copy of cachePaths to avoid mutating the original array as reported by:
        // https://github.com/actions/toolkit/pull/1378
        // TODO: remove this once the underlying bug is fixed.
        await cacheProvider.cache.saveCache(config.cachePaths.slice(), config.cacheKey);
    }
    catch (e) {
        reportError(e);
    }
    process.exit();
}
run();
async function macOsWorkaround() {
    try {
        // Workaround for https://github.com/actions/cache/issues/403
        // Also see https://github.com/rust-lang/cargo/issues/8603
        await exec("sudo", ["/usr/sbin/purge"], { silent: true });
    }
    catch { }
}
