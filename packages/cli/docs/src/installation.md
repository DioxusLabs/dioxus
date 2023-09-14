# Installation

## Install the latest development build through git

To get the latest bug fixes and features, you can install the development version from git.

```
cargo install --git https://github.com/Dioxuslabs/cli
```

This will download `Dioxus-CLI` source from GitHub master branch,
and install it in Cargo's global binary directory (`~/.cargo/bin/` by default).

## Install stable through `crates.io`

The published version of the Dioxus CLI is updated less often, but is more stable than the git version.

```
cargo install dioxus-cli --locked
```

Run `dx --help` for a list of all the available commands.
Furthermore, you can run `dx <COMMAND> --help` to get help with a specific command.
