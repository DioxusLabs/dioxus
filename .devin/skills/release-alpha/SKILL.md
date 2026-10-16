---
name: release-alpha
description: Cut a Dioxus 0.x alpha release (version bump, crates.io publish, tag, GitHub release, CLI binaries, template update, verification, and optionally the SDK/icons/code/components releases)
argument-hint: "<new version, e.g. 0.8.0-alpha.3>"
triggers:
  - user
  - model
allowed-tools:
  - read
  - grep
  - glob
---

# Releasing a Dioxus alpha

Use this to cut a prerelease of the whole workspace (for example `0.8.0-alpha.3`). The general checklist is in `notes/RELEASING.md`. This skill is the concrete, tested procedure for an alpha, written down after cutting `0.8.0-alpha.2`. Step 8 (ecosystem crates) was added after `0.8.0-alpha.3`.

`NEW` is the version being released and `PREV` is the last release tag (e.g. `v0.8.0-alpha.2`). If the user didn't give `NEW`, ask.

## Confirmation gates

These steps can't be undone. Do each one only when the user has explicitly asked for it, and don't treat approval of one as approval of the next:

- publishing to crates.io (versions can't be deleted)
- pushing to `main`. Branch protection requires a PR, and the user's account can bypass that rule, so a direct push goes through silently. Prefer a PR.
- pushing tags, creating the GitHub release, and running the `Publish CLI` workflow
- creating new crates on crates.io and changing crate owners

Never edit `.npmrc`, `minimumReleaseAge` or branch protection to get around a failure.

## Environment notes (macOS)

- **bash 3.2:** a heredoc inside `$(...)` that contains an apostrophe fails to parse. Write commit messages and PR bodies to a file and use `git commit -F file` and `gh pr create --body-file file`.
- **sed:** in-place edits need `sed -i ''`.
- **crates.io API:** requests are rejected without a User-Agent, so pass `curl -A dioxus-release-check ...`.
- **Don't touch the user's `dx`:** don't `cargo install` over it. Build `dx` into `target/release/dx` with `cargo build -p dioxus-cli --release`, and set `DX_HOME=<scratch dir>` so its tools are downloaded fresh and the user's dx data stays untouched.
- **Cleanup:** put scratch work under `/tmp/<name>` and delete it when done.

## 1. Pre-flight

1. **Base commit and CI.** Find the latest `main` commit with `git fetch origin`. Check its CI with `gh run list --branch main --workflow main.yml --limit 3`. Wait for green before cutting.
2. **What's new.** Find the commit `PREV` was cut from: `git merge-base origin/main PREV`. A tag can be on a side branch (alpha.1's was). List the commits since then with `git log --oneline <base>..origin/main`.
3. **New publishable crates.** Diff the `[workspace.dependencies]` entries in `Cargo.toml` against `PREV`. For each publishable member, query `https://crates.io/api/v1/crates/<name>`; a 404 means it's new.
   - **Claim the names first** (confirmation gate). For each new crate, in `/tmp`, run `cargo new --lib --vcs none <name>`. Copy `description`, `license`, `repository`, `homepage`, `keywords` and `authors` from the real crate, set `version = "0.0.0"`, and make `src/lib.rs` just `//! Coming soon.`. Then `cargo publish --dry-run` followed by `cargo publish`.
   - **Add owners:** `github:dioxuslabs:publish`, `jkelleyrtp` (Jonathan Kelley) and `ealmloff` (Evan Almloff), with `cargo owner --add github:dioxuslabs:publish --add jkelleyrtp --add ealmloff <crate>`. This needs a token with the `change-owners` scope. If cargo returns 403, give the user `https://crates.io/crates/<crate>/settings` instead.
   - **Naming:** a crate name can't be changed once it has a real release, so settle names before publishing.
4. **Dev-dependencies on `publish = false` crates** (for example `dioxus-renderer-oracle` in `dioxus-core`). These are fine because `cargo workspaces publish` strips dev-dependencies by default (it logs `warn removing dev-deps ...`). Never pass `--no-remove-dev-deps`. Plain `cargo publish` or `cargo publish --workspace` would fail on them.
5. **Optional packaging dry run.** `cargo workspaces publish --dry-run` can't get past the first crate whose dependencies aren't on crates.io yet, so it isn't useful. Instead:
   - Create a scratch worktree with `git worktree add --detach /tmp/dioxus-dryrun HEAD`.
   - Bump the version there, and strip `[dev-dependencies]` from the crates you're checking.
   - Run `cargo publish --dry-run --no-verify --allow-dirty -p <crate> -p <each workspace crate it depends on, transitively>`.
   - Remove the worktree afterwards with `git worktree remove --force`.
6. **Smoke test** (web, from the base commit):
   - Build `dx` as described in the environment notes.
   - Run `dx new` in `/tmp`. It needs a TTY, and even with `--yes` it still asks for the sub-template and platform. To run it without a TTY, pass `--subtemplate Bare-Bones --option default_platform=web --yes --vcs none`.
   - Add `[patch.crates-io] dioxus = { path = "<repo>/packages/dioxus" }` to the project.
   - Run `dx serve --web --port <p> --open false --interactive false`. In the browser, check that the page renders and the CSS/SVG assets load.
   - Check hot reload: edit RSX text, then CSS, then add a `use_signal` counter button (a Rust hot-patch) and click it.
   - Run `dx bundle --web --release`, serve `target/dx/<app>/release/web/public` with `python3 -m http.server`, and click the button again.
   - Check that `dx` installs `wasm-bindgen-cli` at the same version as the project's `wasm-bindgen`, and compare against the latest on crates.io.
   - To check app logs, use the interactive TUI. `--interactive false --verbose` drops app logs; that's an existing bug.

## 2. Version bump

1. Create the branch: `git switch -c release-NEW origin/main`, then `git branch --unset-upstream`.
2. Find every reference to the old version with `rg -l --glob '!Cargo.lock' --glob '!target/**' '<OLD_VERSION_REGEX>' .`. Expect:
   - `Cargo.toml` (the `[workspace.package]` version and every `=<old>` entry in `[workspace.dependencies]`)
   - `packages/const-serialize/Cargo.toml`
   - `packages/const-serialize-macro/Cargo.toml`
   - `packages/manganis/manganis-core/Cargo.toml` (its `const-serialize` dependency)
3. Replace the version with `sed -i '' 's/<old>/<new>/g' <files>`, then run `cargo update -w`.
4. Verify:
   - In `git diff -U0 Cargo.lock`, the only changed lines are `version = "..."` lines.
   - `cargo metadata --locked` succeeds.
   - `cargo check --locked -p dioxus -p dioxus-cli` succeeds.
5. Commit with the message `Bump versions for NEW release` (the existing convention).
6. Get the bump onto `main` before tagging, preferably through a PR. The tag must end up on a commit that's on `main`.

## 3. Release notes

1. Generate the base notes:
   `gh api repos/DioxusLabs/dioxus/releases/generate-notes -f tag_name=vNEW -f target_commitish=main -f previous_tag_name=PREV --jq .body`.
   `target_commitish` must be a branch name; a SHA is rejected.
2. Above the generated list, add a `## Highlights` section and, if needed, a `## Breaking changes` section. Write them from the PR descriptions (`gh pr view <n> --json title,body`), not just the titles. Keep the generated "What's Changed", "New Contributors" and "Full Changelog" sections unchanged.
3. Credit original authors of rebased or superseded PRs, e.g. `(originally #NNNN by @user)`.
4. Commits pushed straight to main have no PR and won't appear in the generated list.
5. Save the notes to `/tmp/vNEW-release-notes.md`.

## 4. Publish (confirmation gate)

From a clean tree on the bump commit, run the command from `notes/RELEASING.md`:

```
cargo workspaces publish --publish-as-is --allow-dirty --no-git-push --no-individual-tags
```

- **Rate limits:** crates.io allows a burst of about 30 to 34 updates to existing crates, then about 1 per minute. When it returns `429 Too Many Requests ... try again after <date> GMT`, `cargo-workspaces` stops.
- **Retrying:** re-run with `publish-with-retry.sh` in this skill's directory. Each re-run skips crates that are already published, waits until the time crates.io gave, and stops on any non-rate-limit error. It shows each run's output as it happens and also saves it to `<log prefix><run>.log`. Run it in the background and report progress as `published N/total`.
- **Timing:** for about 56 crates, expect about 30 minutes in total.
- **Check the result:** query `https://crates.io/api/v1/crates/<name>/NEW` for every publishable crate. Take the list from `cargo metadata --no-deps` (packages where `publish` is null or non-empty).

## 5. Tag, GitHub release and CLI binaries (each a confirmation gate)

1. **Tag:** previous tags are lightweight. Run `git tag vNEW <bump commit>`, then `git push origin vNEW`.
2. **GitHub release:**
   `gh release create vNEW --verify-tag --prerelease --latest=false --title vNEW --notes-file /tmp/vNEW-release-notes.md`.
   Afterwards, confirm `gh api repos/DioxusLabs/dioxus/releases/latest` still returns the last stable release.
3. **CLI binaries:** the release must exist first, because the workflow uploads to it:
   `gh workflow run publish.yml --ref main -f post=vNEW -f channel=vNEW`.
   It takes about 30 minutes; Intel macOS is the slowest job. When it finishes, the release should have 16 files: `.zip`, `.tar.gz` and `.sha256` for each of 6 targets (Windows has no `.tar.gz`).

## 6. Verify

1. **docs.rs:** check `https://docs.rs/crate/<name>/NEW/status.json` (`doc_status`) for every crate.
   - `dioxus-cli` always fails because it's binary-only. That's expected.
   - For any other failure, read the build log at `https://docs.rs/crate/<name>/NEW/builds/<id>`; the first build id is linked from `/builds`. Also check whether the previous version failed the same way.
2. **binstall:**
   - Download a `cargo-binstall` release that's at least 7 days old into `/tmp` with `gh release download vX --repo cargo-bins/cargo-binstall --pattern 'cargo-binstall-aarch64-apple-darwin.zip'`.
   - Run `cargo-binstall dioxus-cli@NEW --root /tmp/<scratch> --no-confirm --no-track --disable-telemetry --strategies crate-meta-data`.
   - `dx --version` should print `NEW (<tag commit short sha>)`.
3. **Published crates:**
   - Run `dx new` with the binstalled `dx`, without the `[patch]` section.
   - Run `cargo generate-lockfile`, and check that `dioxus*` resolve to `NEW` from the registry.
   - Run `dx serve` and repeat the browser and hot-reload checks from 1.6.

## 7. Template

The default `dx new` template is `gh:dioxuslabs/dioxus-template`, on the branch `v<major>.<minor>` (e.g. `v0.8`).

1. Clone it into `/tmp` with `gh repo clone DioxusLabs/dioxus-template /tmp/dioxus-template -- --branch v0.8`, and create a branch.
2. Bump the old version to `NEW` in:
   - `common.rhai` (`DIOXUS_DEP_SRC_VAR`)
   - `Bare-Bones/AGENTS.md`, `Jumpstart/AGENTS.md`, `Workspace/AGENTS.md`
   - `.github/workflows/main.yml` (the `cargo binstall dioxus-cli@...` line)
3. Leave `dioxuslabs.com/learn/<ver>/` links alone unless that docs version returns 200.
4. Test locally the way the template's CI does: for each of Bare-Bones, Jumpstart and Workspace, run `dx new --template . --subtemplate <X> --yes --option default_platform=web --vcs none <dir>` (Workspace doesn't need `default_platform`), then `cargo check --all-features`.
5. Open a PR against `v0.8`.

## 8. Ecosystem crates (optional)

Only do this if the user asks. These repos pin Dioxus exactly (`=NEW`), so each one needs a matching release. Their 0.8 work lives on a `0.8` branch (`v<major>.<minor>` without the `v`); `main` stays on 0.7.

| Repo | Crates published | Version |
|---|---|---|
| `DioxusLabs/sdk` | `dioxus-sdk` and the `dioxus-sdk-{time,storage,geolocation,notification,sync,util,window}` subcrates | `NEW` (mirrors Dioxus) |
| `DioxusLabs/dioxus-icons` | `dioxus-icons` | its next minor with Dioxus's prerelease suffix, e.g. `0.2.0-alpha.3` |
| `DioxusLabs/dioxus-code` | `dioxus-code-macro`, `dioxus-code`, `dioxus-code-editor` (all one version) | same rule as icons |
| `DioxusLabs/dioxus-components` | nothing; `dx components add` uses the `0.8` branch from git | n/a |

Prerelease versions mean `cargo add` keeps giving 0.7 users the latest stable release.

1. **Look for upgrade work already in progress.** Run `gh pr list --repo <repo> --state open` on each repo. If a contributor has an upgrade branch open, build the release PR on top of their commits rather than starting fresh. Add `Co-authored-by: <name> <email>` (from `git log`) to every commit you make in that repo. Keep their PRs open until yours merge, then close them with a thank-you that links the replacement.
2. **Release order.** Components depends on the SDK (`dioxus-primitives` → `dioxus-sdk-time`), and its unpublished `preview` crate depends on icons and code. The dioxus-code docsite depends on `dioxus-primitives` from components via git. So:
   1. Release the SDK and icons: PR to `0.8` (create the branch from `main` if it doesn't exist), merge, publish, tag.
   2. Open the components PR to `0.8`. Until the crates above are published, point it at their release branches as git dependencies. Merge it.
   3. Release dioxus-code. Pin the docsite to the components merge commit with `rev = "<sha>"`, not `branch = "0.8"`. Because both repos pin Dioxus exactly, a branch reference breaks resolution as soon as components moves to the next alpha.
   4. Follow-up PR in components: switch to exact pins on the published crates (`=NEW`, `=0.x.0-alpha.N`). Check that `Cargo.lock` has no `git+` sources left.
3. **Each release PR:** bump the Dioxus pins and the crate versions, update the README install lines and any compatibility notes, then run the repo's CI commands locally. CI on these repos usually only runs for PRs into `main`, so it won't run on a PR into `0.8` (components#307 adds version-branch CI there). Expect API fixes from that Dioxus release, such as the keyboard-types 0.8 `Key::Named(NamedKey::…)` change in alpha.3.
4. **Publish** (confirmation gate). Publish from a clean `git worktree add --detach <dir> <merge commit>`:
   - Check `git diff --stat <tested head> <merge commit>` is empty (PRs are squash-merged) and the squash commit kept the `Co-authored-by` trailers.
   - Check `cargo workspaces list` shows only the crates above, with no examples.
   - Run `publish-with-retry.sh <worktree> <log prefix>`. Use `cargo workspaces` for dioxus-code too: `dioxus-code-macro` dev-depends on `dioxus-code`, and plain `cargo publish` can't handle that cycle.
5. **Tag and release** (confirmation gates): add a lightweight `v<version>` tag on the merge commit, then run `gh release create ... --prerelease --latest=false`. Above GitHub's generated notes (`target_commitish=0.8`, `previous_tag_name=<last tag>`), add a short intro: the Dioxus version it targets, the install line, a note that 0.7 users aren't affected, and credit for the PRs it builds on. The generated list only credits whoever opened the release PR.
6. **Verify:**
   - every version returns 200 from the crates.io API, and `max_stable_version` is unchanged
   - a scratch project that depends on Dioxus `NEW` plus the new crates resolves everything from the registry, with one `dioxus`, and passes `cargo check`
   - docs.rs builds. `dioxus-sdk` and `dioxus-sdk-window` fail because `listen` and `get_theme_platform` are defined twice under all features (0.7.0 failed the same way). Large crates like `dioxus-icons` take a few minutes to show as built.
7. **Gotchas:**
   - The components `preview` and dioxus-code docsite web builds compile C to wasm32 (`arborium-sysroot`), which Apple clang can't do. Without an LLVM clang, check those for desktop instead.
   - None of these repos delete merged branches automatically. Delete the temporary release branches only once nothing references them, including the docsite's components rev.

## 9. Wrap up

Report back:

- links to the crates.io release, the GitHub release, the CLI workflow run and any PRs
- the docs.rs results
- anything you didn't test (desktop, mobile, fullstack, Windows/Linux)
- follow-ups: owner invites, open PRs, letting VIP projects like Freya know, and the Discord release-notes channel

Delete all `/tmp` scratch files and worktrees you created.
