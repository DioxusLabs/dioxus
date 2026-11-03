// Remove build artifacts that the current job did not use from a restored cargo target dir.
//
// rust-cache restores the newest cache for a job even when Cargo.lock has changed, and its own
// cleanup keeps artifacts by crate name, so outputs for old `-<hash>`es pile up in every save.
//
//   mark(target)   run after restoring the cache: set the atime of every fingerprint file to 0
//   prune(target)  run after the build: delete every unit whose fingerprint cargo did not read
//
// Cargo reads the fingerprint of every unit in the build graph (fresh or not) and ignores atime, so
// under `relatime` any unit still at atime 0 after the build was not part of it. If no fingerprint
// was read at all, or reading an unread one does not update its atime (e.g. a `noatime` mount),
// nothing is deleted.

const fs = require("fs");
const path = require("path");

const UNIT_RE = /^.+-([0-9a-f]{16})(\..*)?$/;
const UNIT_DIRS = ["deps", "build", ".fingerprint", "examples"];

function subdirs(dir) {
  return fs
    .readdirSync(dir, { withFileTypes: true })
    .filter((e) => e.isDirectory())
    .map((e) => e.name);
}

function* profileDirs(dir) {
  const dirs = subdirs(dir);
  const isProfile = dirs.includes(".fingerprint");
  if (isProfile) yield dir;
  for (const d of dirs) {
    if (isProfile && (UNIT_DIRS.includes(d) || d === "incremental")) continue;
    yield* profileDirs(path.join(dir, d));
  }
}

function* fingerprintUnits(profile) {
  const fingerprints = path.join(profile, ".fingerprint");
  for (const name of subdirs(fingerprints)) {
    const unit = path.join(fingerprints, name);
    yield [name, fs.readdirSync(unit).map((f) => path.join(unit, f))];
  }
}

function mark(target) {
  let count = 0;
  for (const profile of profileDirs(target)) {
    for (const [, files] of fingerprintUnits(profile)) {
      for (const f of files) {
        const mtime = Number(fs.lstatSync(f, { bigint: true }).mtimeNs) / 1e9;
        fs.lutimesSync(f, 0, mtime);
        count++;
      }
    }
  }
  console.log(`Marked ${count} fingerprint files in ${target}`);
}

function size(p) {
  const stat = fs.lstatSync(p);
  if (!stat.isDirectory()) return stat.size;
  let total = 0;
  for (const e of fs.readdirSync(p, { withFileTypes: true })) {
    const child = path.join(p, e.name);
    total += e.isDirectory() ? size(child) : fs.lstatSync(child).size;
  }
  return total;
}

function prune(target) {
  const profiles = [...profileDirs(target)];
  const live = new Map();
  let unread;
  for (const profile of profiles) {
    const hashes = new Set();
    for (const [name, files] of fingerprintUnits(profile)) {
      const m = UNIT_RE.exec(name);
      let read = false;
      for (const f of files) {
        const st = fs.lstatSync(f, { bigint: true });
        if (st.atimeNs > 0n) read = true;
        else if (!unread && st.size > 0n) unread = f;
      }
      if (m && read) hashes.add(m[1]);
    }
    live.set(profile, hashes);
  }
  if (![...live.values()].some((hashes) => hashes.size > 0)) {
    console.log(`No fingerprints in ${target} were read since \`mark\`; not pruning`);
    return;
  }
  if (unread) {
    fs.readFileSync(unread);
    if (fs.lstatSync(unread, { bigint: true }).atimeNs === 0n) {
      console.log(`Reading ${unread} did not update its atime; not pruning`);
      return;
    }
  }

  for (const profile of profiles) {
    let removed = 0;
    let freed = 0;
    for (const sub of UNIT_DIRS) {
      const parent = path.join(profile, sub);
      if (!fs.existsSync(parent)) continue;
      for (const name of fs.readdirSync(parent)) {
        const m = UNIT_RE.exec(name);
        if (!m || live.get(profile).has(m[1])) continue;
        const p = path.join(parent, name);
        freed += size(p);
        fs.rmSync(p, { recursive: true, force: true });
        removed++;
      }
    }
    const kept = live.get(profile).size;
    console.log(`${profile}: kept ${kept} units, removed ${removed} entries (${Math.round(freed / 1e6)} MB)`);
  }
}

function run(mode) {
  const target = process.env["INPUT_TARGET-DIR"] || process.argv[2] || "target";
  if (!fs.existsSync(target)) {
    console.log(`${target} does not exist; nothing to ${mode.name}`);
    return;
  }
  mode(target);
}

module.exports = { mark, prune, run };
