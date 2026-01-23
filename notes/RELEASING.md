# Releasing Dioxus

Dioxus is big and requires a lot of hand-holding to release. This guide is accurate as of Dec 17, 2024. As we improve the release process, we'll update this guide.

## Checklist

What we need to release:
- [ ] The dioxus crates in the workspace
- [ ] The docsite and its nightly docs
- [ ] The VSCode Extension
- [ ] Any supporting crates (sdk, include_mdbook, etc)

### Prepping for the release

This involves making sure the metadata of the crates is correct before we publish.

1. [ ] Draft a release post for GitHub (but don't publish it yet)
2. [ ] Make sure every crate has a *complete* Cargo.toml
3. [ ] Make sure shared dependencies are hoisted to the workspace
4. [ ] Pin dependencies that are known to break semver
5. [ ] Make sure no dependencies are `git` or `path` dependencies - all dependencies should be versioned
6. [ ] Make sure all workspace members are listed in the members array of the workspace
7. [ ] Make sure all workspace members are listed in the workspace dependencies list
8. [ ] Make sure all "author" fields are correct
9. [ ] Make sure no assets or READMEs are pointed to outside their respective crates
10. [ ] Run `cargo doc --workspace --no-deps --all-features --document-private-items` to make sure docs can compile
11. [ ] Look through the docs to ensure nothing is glaringly missing or broken
12. [ ] Ensure the crate actually builds the features listed in its docs.rs platform list (we've had [issues where dioxus doesn't build on wasm with --all-features](https://github.com/DioxusLabs/dioxus/issues/3381))
13. [ ] Make sure all the workspace crates are actually owned by either Jonathan Kelley or the Dioxus Labs publish org (`cargo owner --add github:dioxuslabs:publish <crate>`)
14. [ ] If there's *any* crates used by dioxus but not in dioxuslabs/dioxus make sure they're owned by the Dioxus Labs github org in case we need to fix them later
15. [ ] Run through all the examples and make sure they compile and actually *work*. `cargo run --example <example>` and `dx run <example>` should both work.
16. [ ] Install the current CLI version and make sure it works (`cargo install --path packages/cli`). Ideally `cargo update` as well since `crates.io` doesn't use a locked install.
17. [ ] Update the dioxus crate versions to be the intended release version. We set all the versions *manually* instead of relying on `cargo workspaces`. This involves going to `Cargo.toml` and updating the `[workspace.package]` version and each crate's version to the new version in the [workspace.dependencies] section.
18. [ ] Go to the [template repo](http://github.com/dioxusLabs/dioxus-template) and make sure a branch exists for the new major version. IE a v0.5, v0.6, v0.7, etc branch. If it doesn't exist, create it. This *needs* to exist for `dx new` to work. It's likely that this already exists since we tend to try `dx new` frequently before releasing.
19. [ ] If performing a major release, make sure all the links in `dioxus` are updated to point to the new version. This involves basically CTRL-F'ing for `/0.6/` and replacing it with `/0.7/` etc.
20. [ ] It's likely that docsite links might not be updated just yet. If nightly docs are released, there shouldn't need to be any changes. It's fine to bypass the link checker for now, but you should be ready to fix links once the docsite is ready. Any links that are broken are "frozen in time" and will need to be fixed. We can't change links in published crates, so if the link never exists, it's just broken forever.
21. [ ] Inform "VIP" community projects that a final RC is out (i.e., projects like [Freya](http://freyaui.dev)) so their authors can test new versions.

### Releasing the workspace:

1. [ ] Make sure all latent commits have been merged into the `main` branch and pushed to github.
2. [ ] Ensure you've published a pre-release of the same code (only necessary for major releases... patch releases are generally fine to skip a prerelease)
3. [ ] Make sure the version you're releasing is correct (see above)
4. [ ] Make sure you're on the `main` branch (cargo workspaces publish requires you to be on the main branch)
5. [ ] Make sure you have [`cargo-workspaces` installed](https://crates.io/crates/cargo-workspaces). There are other tools but this one is the one we've used in the past. It has some small bugs but is generally reliable. This tool is important because it coordinates the release order of the crates since they depend on each other. Eventually cargo itself will have this functionality, I believe. Unfortunately, there's no way to "dry-run" a workspace publish since crates rely on each other and won't succeed if their dependencies aren't published.
6. [ ] Run the release: `cargo workspaces publish --publish-as-is --allow-dirty --no-git-push --no-individual-tags`. This will publish the crates to crates.io. It might take a while. Only `jkelleyrtp` currently has sufficient rate-limits to publish all the crates at once. If any crate fails, you might need to fix the problems manually and then run the command again. If an error occurs, you might also need to reset the most recent git commit and wipe the tag. `git reset --hard HEAD~1` and `git tag -d <tag>`. Be careful with these commands, especially if you're on the `main` branch.
7. [ ] Once the release is up, commit the most recent changes to the `main` branch and push it.
8. [ ] Also push the tag to the `main` branch, e.g., `git push origin v0.6.0`
9. [ ] Verify crates.io is showing the new version
10. [ ] Verify `docs.rs` builds the new docs for each crate. IE go to `https://docs.rs/crate/dioxus/latest` and ensure there's no errors. We've had issues before with docs.rs [not building properly](https://docs.rs/crate/dioxus/0.6.0).
11. [ ] Verify you can create a new project with the new version and it works. IE `dx new app`. Do a dry-run of building a new app to make sure it works and no obvious errors are present.
12. [ ] Release the GitHub release using the tag we pushed earlier.
13. [ ] Execute the [`Publish CLI` github action](https://github.com/DioxusLabs/dioxus/actions/workflows/publish.yml) using a manual trigger. Fill in the small form with the appropriate information. This should be the version you just released IE `v0.6.1`. The corresponding github release post must exist for the binstall to be published! You need to be part of the dioxuslabs/publish org to trigger this action.
14. [ ] If you're about to start working on a "dev" version of Dioxus, create a new branch for the last version that we backport fixes to. IE the dioxus repo has a v0.4, v0.5, v0.6, etc branch. We generally only create this branch when we're ready to start merging breaking PRs.

### Releasing the docsite

1. [ ] Stabilize the current docs version on the docsite. This will be manual, unfortunately, and involves changing URLs in the navbar and the sidebar. See [this PR for an example for v0.6](https://github.com/DioxusLabs/docsite/pull/342).
2. [ ] Make sure to update the "current version" and any stability bools in the version switcher.
3. [ ] Update the `deploy.yml` to point to the new binstall version
4. [ ] Remove any latent `git` or `path` dependencies and `crates.io` patches.
5. [ ] todo: discourage any old versions of dioxus from ending up in robots.txt - ideally for better AI support
6. [ ] Move any non-working examples to the relevant `examples/untested` folder.
7. [ ] Update the current `dioxus` version in Cargo.toml (use the docsite as a canary)
8. [ ] run `dx serve` and make sure it works
9. [ ] Commit to main and ensure the build and checks CI passes
10. [ ] Ensure that `ssg` is properly generating *all* the pages. Currently it's flakey and occasionally fails.
11. [ ] Ensure google analytics is working. Check the console and make sure we haven't "gone silent."
12. [ ] Double-check that we're generating OpenGraph images. Twitter/Discord/Reddit/etc will use these images.

### Releasing the vscode extension

This is very manual. In theory it can be automated but I've struggled with the azure portal in the past.

1. [ ] Update the version in `package.json` and `package-lock.json`
2. [ ] Make sure `wasm-bindgen` is installed to current version (`cargo binstall wasm-bindgen-cli`)
3. [ ] Run `npm install` to make sure you install all the dependencies
4. [ ] run `npm run vsix` to create the vsix file
5. [ ] Either use `vsce publish` or manually upload the vsix file to the marketplace

### Releasing ecosystem crates

There are a number of crates that are part of the Dioxus ecosystem that we usually want ready before publishing to reddit/twitter/youtube/etc.

1. [ ] [dioxus-sdk](https://github.com/DioxusLabs/sdk): folks use this and we like to release it with the rest of the ecosystem
2. [ ] [blitz](https://github.com/DioxusLabs/blitz): integrates with dioxus-native and thus dioxus-native needs to be version matched
3. [ ] [taffy](https://github.com/DioxusLabs/taffy): usually exists on its own version system and is used by major projects like Bevy and Zed
4. [ ] [icons](https://github.com/dioxus-community/dioxus-free-icons): a community crate that is usually updated by marc
5. [ ] [use-mounted](https://crates.io/crates/dioxus-use-mounted): a community crate that is usually updated by marc
6. [ ] [resize-observer](https://crates.io/crates/dioxus-resize-observer): a community crate that is usually updated by marc
7. [ ] [charts](https://crates.io/crates/dioxus-charts): a community crate that is usually updated by marc
8. [ ] [lazy](https://crates.io/crates/dioxus-lazy): a community crate that is usually updated by marc
9. [ ] [free-icons](https://crates.io/crates/dioxus-free-icons): a community crate that is usually updated by marc
10. [ ] [query](https://crates.io/crates/dioxus-query): a community crate that is usually updated by marc
11. [ ] [radio](https://crates.io/crates/dioxus-radio): a community crate that is usually updated by marc
12. [ ] [i18n](https://crates.io/crates/dioxus-i18n): a community crate that is usually updated by marc
13. [ ] [clipboard](https://crates.io/crates/dioxus-clipboard): a community crate that is usually updated by marc
14. [ ] [use-computed](https://crates.io/crates/dioxus-use-computed): a community crate that is usually updated by marc
15. [ ] there might be more...


## Marketing

Verify everything works and is ready to go.
- [ ] crates.io is up to date
- [ ] docs.rs is up to date and has no failing builds for major crates (dioxus, dioxus-desktop, etc)
- [ ] github release post is out
- [ ] The relevant GitHub tag is created
- [ ] Any fixes are backported to the relevant branches.
- [ ] binstalls are published to the github release post.
- [ ] `dx new app` plus `dx serve` works out of the box. hot-reload works for rsx and assets.
- [ ] No `wasm-bindgen` errors popup, especially with version bumps.
- [ ] YouTube video is uploaded
- [ ] Ensure open-graph images are generated. Ideally we add custom open-graph images per-release.

Actually marketing:
- [ ] Publish to reddit (usually early morning monday)
- [ ] Publish to twitter
- [ ] Publish to discord in the release-notes channel
- [ ] Publish the youtube video
- [ ] (sometimes) publish to hackernews

## Notes, todos, etc

- We manually bump versions of the crates in the workspace. I don't really trust it to bump automatically, but in theory it is able to do it automatically using the git history. A challenge is crates relying on versions of each other - like core (0.6.2) relying on html (0.6.1)  or something like that. This means our releases release the entire crate with no changes, but it does keep every crate in sync with each other.
- We need to expand the rate limit on crates.io for `github:dioxuslabs:publish` to be able to publish all the crates at once.
- We should make stabilizing the docs easier. It's quite manual now.
- We should automate the release of the vscode extension.
- We move examples to the `examples/untested` folder when we release a new version. Ideally we have a workspace with all versions of dioxus and can include the examples in the relevant frames.
- SSG is flakey and occasionally fails. This needs to be fixed.
- We should promote "community" crates to enjoy auto-updates via codemods whenever a new version is released.
- We should be running `docs` CI using DOCS_RS env var to ensure docs are built the same way as docs.rs
