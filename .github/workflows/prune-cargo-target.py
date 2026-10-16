#!/usr/bin/env python3
"""Remove build artifacts that the current job did not use from a restored cargo target dir.

rust-cache restores the newest cache for a job even when Cargo.lock has changed, and its own
cleanup keeps artifacts by crate name, so outputs for old `-<hash>`es pile up in every save.

    mark  <target-dir>   run after restoring the cache: set the atime of every fingerprint file to 0
    prune <target-dir>   run after the build: delete every unit whose fingerprint cargo did not read

Cargo reads the fingerprint of every unit in the build graph (fresh or not) and ignores atime, so
under `relatime` any unit still at atime 0 after the build was not part of it. If no fingerprint
was read at all (e.g. a `noatime` mount) nothing is deleted.
"""

import os
import re
import shutil
import sys

UNIT_RE = re.compile(r"^.+-([0-9a-f]{16})(\..*)?$")
UNIT_DIRS = ("deps", "build", ".fingerprint", "examples")


def profile_dirs(target):
    for root, dirs, _ in os.walk(target):
        if ".fingerprint" in dirs:
            yield root
            dirs[:] = [d for d in dirs if d not in UNIT_DIRS and d != "incremental"]


def fingerprint_units(profile):
    fingerprints = os.path.join(profile, ".fingerprint")
    for name in os.listdir(fingerprints):
        unit = os.path.join(fingerprints, name)
        if os.path.isdir(unit):
            yield name, [os.path.join(unit, f) for f in os.listdir(unit)]


def mark(target):
    count = 0
    for profile in profile_dirs(target):
        for _, files in fingerprint_units(profile):
            for f in files:
                os.utime(f, ns=(0, os.stat(f).st_mtime_ns), follow_symlinks=False)
                count += 1
    print(f"Marked {count} fingerprint files in {target}")


def size(path):
    if os.path.islink(path) or not os.path.isdir(path):
        return os.lstat(path).st_size
    return sum(os.lstat(os.path.join(r, f)).st_size for r, _, fs in os.walk(path) for f in fs)


def prune(target):
    profiles = list(profile_dirs(target))
    live = {}
    for profile in profiles:
        live[profile] = {
            UNIT_RE.match(name).group(1)
            for name, files in fingerprint_units(profile)
            if UNIT_RE.match(name) and any(os.lstat(f).st_atime_ns > 0 for f in files)
        }
    if not any(live.values()):
        print(f"No fingerprints in {target} were read since `mark`; not pruning")
        return

    for profile in profiles:
        removed = freed = 0
        for sub in UNIT_DIRS:
            parent = os.path.join(profile, sub)
            if not os.path.isdir(parent):
                continue
            for name in os.listdir(parent):
                m = UNIT_RE.match(name)
                if not m or m.group(1) in live[profile]:
                    continue
                path = os.path.join(parent, name)
                freed += size(path)
                if os.path.isdir(path) and not os.path.islink(path):
                    shutil.rmtree(path)
                else:
                    os.remove(path)
                removed += 1
        print(f"{profile}: kept {len(live[profile])} units, removed {removed} entries ({freed / 1e6:.0f} MB)")


if __name__ == "__main__":
    mode, target = sys.argv[1], sys.argv[2]
    if not os.path.isdir(target):
        print(f"{target} does not exist; nothing to {mode}")
    elif mode == "mark":
        mark(target)
    elif mode == "prune":
        prune(target)
    else:
        sys.exit(f"unknown mode {mode!r}")
