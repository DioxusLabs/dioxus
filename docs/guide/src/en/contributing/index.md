# Contributing

Development happens in the [Dioxus GitHub repository](https://github.com/DioxusLabs/dioxus). If you've found a bug or have an idea for a feature, please submit an issue (but first check if someone hasn't [done it already](https://github.com/DioxusLabs/dioxus/issues)).

[GitHub discussions](https://github.com/DioxusLabs/dioxus/discussions) can be used as a place to ask for help or talk about features. You can also join [our Discord channel](https://discord.gg/XgGxMSkvUM) where some development discussion happens.

## Improving Docs

If you'd like to improve the docs, PRs are welcome! Both Rust docs ([source](https://github.com/DioxusLabs/dioxus/tree/master/packages)) and this guide ([source](https://github.com/DioxusLabs/dioxus/tree/master/docs/guide)) can be found in the GitHub repo.

## Working on the Ecosystem

Part of what makes React great is the rich ecosystem. We'd like the same for Dioxus! So if you have a library in mind that you'd like to write and many people would benefit from, it will be appreciated. You can [browse npm.js](https://www.npmjs.com/search?q=keywords:react-component) for inspiration. Once you are done, add your library to the [awesome dioxus](https://github.com/DioxusLabs/awesome-dioxus) list or share it in the `#I-made-a-thing` channel on [Discord](https://discord.gg/XgGxMSkvUM).

## Bugs & Features

If you've fixed [an open issue](https://github.com/DioxusLabs/dioxus/issues), feel free to submit a PR! You can also take a look at [the roadmap](./roadmap.md) and work on something in there. Consider [reaching out](https://discord.gg/XgGxMSkvUM) to the team first to make sure everyone's on the same page, and you don't do useless work!

All pull requests (including those made by a team member) must be approved by at least one other team member.
Larger, more nuanced decisions about design, architecture, breaking changes, trade-offs, etc. are made by team consensus.

## Tools

The following tools can be helpful when developing Dioxus. Many of these tools are used in the CI pipeline. Running them locally before submitting a PR instead of waiting for CI can save time.

- All code is tested with [cargo test](https://doc.rust-lang.org/cargo/commands/cargo-test.html)

```sh
cargo fmt --all
```

- All code is formatted with [rustfmt](https://github.com/rust-lang/rustfmt)

```sh
cargo check --workspace --examples --tests
```

- All code is linted with [Clippy](https://doc.rust-lang.org/clippy/)

```sh
cargo clippy --workspace --examples --tests -- -D warnings
```

- Browser tests are automated with [Playwrite](https://playwright.dev/docs/intro#installing-playwright)

```sh
npx playwright test
```

- Crates that use unsafe are checked for undefined behavior with [MIRI](https://github.com/rust-lang/miri). MIRI can be helpful to debug what unsafe code is causing issues. Only code that does not interact with system calls can be checked with MIRI. Currently, this is used for the two MIRI tests in `dioxus-core` and `dioxus-native-core`.

```sh
cargo miri test --package dioxus-core --test miri_stress
cargo miri test --package dioxus-native-core --test miri_native
```

- [Rust analyzer](https://rust-analyzer.github.io/) can be very helpful for quick feedback in your IDE.
