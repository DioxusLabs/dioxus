import { e as error, g as getCacheProvider, a as getInput, b as exportVariable, C as CacheConfig, i as info, c as cleanTargetDir, r as reportError, s as setOutput } from './cleanup-C5dkQaqv.js';
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
    if (!cacheProvider.cache.isFeatureAvailable()) {
        setCacheHitOutput(false);
        return;
    }
    try {
        var cacheOnFailure = getInput("cache-on-failure").toLowerCase();
        if (cacheOnFailure !== "true") {
            cacheOnFailure = "false";
        }
        var lookupOnly = getInput("lookup-only").toLowerCase() === "true";
        exportVariable("CACHE_ON_FAILURE", cacheOnFailure);
        exportVariable("CARGO_INCREMENTAL", 0);
        const config = await CacheConfig.new();
        config.printInfo(cacheProvider);
        info("");
        info(`... ${lookupOnly ? "Checking" : "Restoring"} cache ...`);
        const key = config.cacheKey;
        // Pass a copy of cachePaths to avoid mutating the original array as reported by:
        // https://github.com/actions/toolkit/pull/1378
        // TODO: remove this once the underlying bug is fixed.
        const restoreKey = await cacheProvider.cache.restoreCache(config.cachePaths.slice(), key, [config.restoreKey], {
            lookupOnly,
        });
        if (restoreKey) {
            const match = restoreKey.localeCompare(key, undefined, {
                sensitivity: "accent",
            }) === 0;
            info(`${lookupOnly ? "Found" : "Restored from"} cache key "${restoreKey}" full match: ${match}.`);
            if (!match) {
                // pre-clean the target directory on cache mismatch
                for (const workspace of config.workspaces) {
                    try {
                        await cleanTargetDir(workspace.target, [], true);
                    }
                    catch { }
                }
                // We restored the cache but it is not a full match.
                config.saveState();
            }
            setCacheHitOutput(match);
        }
        else {
            info("No cache found.");
            config.saveState();
            setCacheHitOutput(false);
        }
    }
    catch (e) {
        setCacheHitOutput(false);
        reportError(e);
    }
    process.exit();
}
function setCacheHitOutput(cacheHit) {
    setOutput("cache-hit", cacheHit.toString());
}
run();
