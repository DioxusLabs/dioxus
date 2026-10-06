#!/usr/bin/env python3
"""
Classify the public-API impact of a PR with cargo-semver-checks.

Two questions are answered for every publishable library crate in the workspace:

  1. PR vs. main       — does this PR remove/alter API (breaking) or add API (expanding)?
  2. main vs. crates.io — has main *already* broken compatibility with the last published
                          version? (only asked for crates the PR breaks)

A PR is only failed when it introduces a breaking change to a crate that is still
semver-compatible with its last crates.io release ("fresh break"), because that is
the change that closes the door on shipping main as a patch release. Breaking a
crate that main has already broken is reported but allowed. API expansion is always
allowed but reported, since it can't be backported in a patch.

Usage:
  semver-check.py --baseline-root <path-to-main-checkout> [--allow-breaking] [--json out.json]

Environment:
  GITHUB_STEP_SUMMARY  if set, a markdown report is appended to it.
"""

import argparse
import json
import os
import re
import subprocess
import sys
from dataclasses import dataclass, field, asdict

EXIT_OK = 0
EXIT_SEMVER_VIOLATION = 100

RE_CHECKING = re.compile(r"^\s*Checking (\S+) v(\S+) -> v(\S+)")
RE_FINISHED = re.compile(r"^\s*Finished \[\s*[\d.]+s\] (\S+)")
RE_FAILED = re.compile(
    r"semver requires new (major|minor|patch) version: (\d+) major and (\d+) minor checks failed"
)
RE_WARNED = re.compile(r"produced (\d+) major and (\d+) minor level warnings")
RE_LINT = re.compile(r"^--- (failure|warning) (\S+): (.*) ---$")


@dataclass
class CrateResult:
    name: str
    # against main
    major: int = 0
    minor: int = 0
    lints: list = field(default_factory=list)
    # against last published version; None = not checked, "unpublished" = no baseline on crates.io
    main_diverged: object = None

    @property
    def classification(self) -> str:
        if self.major:
            return "breaking"
        if self.minor:
            return "expanding"
        return "compatible"


def run(cmd: list[str], **kwargs) -> subprocess.CompletedProcess:
    print("+", " ".join(cmd), flush=True)
    return subprocess.run(cmd, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, **kwargs)


def semver_checks(extra: list[str]) -> subprocess.CompletedProcess:
    cmd = ["cargo", "semver-checks", "check-release", "--release-type", "patch", "--color", "never", *extra]
    proc = run(cmd)
    sys.stdout.write(proc.stdout)
    sys.stdout.flush()
    return proc


def parse(output: str) -> dict[str, CrateResult]:
    """Parse cargo-semver-checks human output into per-crate results."""
    results: dict[str, CrateResult] = {}
    current: CrateResult | None = None
    for line in output.splitlines():
        m = RE_CHECKING.match(line)
        if m:
            current = results.setdefault(m.group(1), CrateResult(m.group(1)))
            continue
        if current is None:
            continue
        m = RE_LINT.match(line.strip())
        if m:
            current.lints.append({"kind": m.group(1), "id": m.group(2), "summary": m.group(3)})
            continue
        m = RE_FAILED.search(line)
        if m:
            current.major += int(m.group(2))
            current.minor += int(m.group(3))
            continue
        m = RE_WARNED.search(line)
        if m:
            # warn-level lints don't fail the tool but still tell us the API moved
            current.major += int(m.group(1))
            current.minor += int(m.group(2))
            continue
        if RE_FINISHED.match(line):
            current = None
    return results


def check_tool_error(proc: subprocess.CompletedProcess, what: str) -> None:
    if proc.returncode not in (EXIT_OK, EXIT_SEMVER_VIOLATION):
        print(f"::error::cargo-semver-checks failed while checking {what} (exit {proc.returncode})")
        sys.exit(proc.returncode or 1)


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--baseline-root", required=True, help="checkout of the base branch (main)")
    ap.add_argument("--allow-breaking", action="store_true", help="report fresh breaks but don't fail")
    ap.add_argument("--json", help="write machine-readable results here")
    ap.add_argument("-p", "--package", action="append", default=[], help="only check these crates (default: whole workspace)")
    args = ap.parse_args()

    # 1. PR vs main, whole workspace (cargo-semver-checks skips publish = false and bin/proc-macro crates)
    selection = [a for p in args.package for a in ("-p", p)] or ["--workspace"]
    proc = semver_checks([*selection, "--baseline-root", args.baseline_root])
    check_tool_error(proc, "the PR against main")
    results = parse(proc.stdout)

    # 2. For crates the PR breaks: has main already broken with the last crates.io release?
    breaking = [r for r in results.values() if r.classification == "breaking"]
    for r in breaking:
        proc = semver_checks(
            ["--manifest-path", os.path.join(args.baseline_root, "Cargo.toml"), "-p", r.name]
        )
        if proc.returncode == EXIT_SEMVER_VIOLATION:
            r.main_diverged = parse(proc.stdout).get(r.name, CrateResult(r.name)).major > 0
        elif proc.returncode == EXIT_OK:
            r.main_diverged = False
        elif "not found in registry" in proc.stdout:
            r.main_diverged = "unpublished"
        else:
            check_tool_error(proc, f"main's {r.name} against crates.io")

    fresh_breaks = [r for r in breaking if r.main_diverged is False]
    expanding = [r for r in results.values() if r.classification == "expanding"]

    # 3. Report
    lines = ["## Semver report", ""]
    if fresh_breaks:
        lines.append(
            f"### :rotating_light: Breaking change against the last published release "
            f"({len(fresh_breaks)} crate{'s' if len(fresh_breaks) != 1 else ''})"
        )
        lines.append("")
        lines.append(
            "main was still semver-compatible with crates.io for these crates; merging this PR "
            "means they can no longer ship as a patch release."
        )
    elif breaking:
        lines.append(":warning: Breaking against main, but main had already diverged from crates.io for these crates.")
    elif expanding:
        lines.append(":large_blue_circle: No breaking changes. Public API was expanded (fine, but not backportable in a patch).")
    else:
        lines.append(":white_check_mark: No public API changes detected.")
    lines.append("")

    interesting = [r for r in results.values() if r.classification != "compatible"]
    if interesting:
        lines += ["| Crate | vs. main | main vs. crates.io | Lints |", "|---|---|---|---|"]
        for r in sorted(interesting, key=lambda r: (r.classification != "breaking", r.name)):
            vs_main = {
                "breaking": f"**breaking** ({r.major} major, {r.minor} minor)",
                "expanding": f"expanding ({r.minor} minor)",
            }[r.classification]
            diverged = {
                None: "—",
                True: "already diverged",
                False: "**compatible → this PR breaks it**",
                "unpublished": "not on crates.io",
            }[r.main_diverged]
            lint_ids = ", ".join(sorted({f"`{l['id']}`" for l in r.lints}))
            lines.append(f"| `{r.name}` | {vs_main} | {diverged} | {lint_ids} |")
        lines.append("")
    lines.append(f"Checked {len(results)} crates with cargo-semver-checks (PR vs. merge-base of main).")
    report = "\n".join(lines)
    print(report)

    summary_path = os.environ.get("GITHUB_STEP_SUMMARY")
    if summary_path:
        with open(summary_path, "a") as f:
            f.write(report + "\n")

    if args.json:
        with open(args.json, "w") as f:
            json.dump(
                {
                    "verdict": "fresh-break" if fresh_breaks else "breaking" if breaking else "expanding" if expanding else "compatible",
                    "crates": [asdict(r) | {"classification": r.classification} for r in results.values()],
                    "report": report,
                },
                f,
                indent=2,
            )

    for r in breaking:
        level = "error" if r.main_diverged is False and not args.allow_breaking else "warning"
        print(f"::{level}::{r.name}: breaking public API change ({', '.join(sorted({l['id'] for l in r.lints}))})")
    for r in expanding:
        print(f"::notice::{r.name}: public API expanded ({', '.join(sorted({l['id'] for l in r.lints}))})")

    if fresh_breaks and not args.allow_breaking:
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
