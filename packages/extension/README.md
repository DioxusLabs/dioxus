# Dioxus VSCode Extension

![Dioxus Splash](https://github.com/DioxusLabs/dioxus/raw/main/notes/dioxus_splash_8.avif)

This extension wraps functionality in Dioxus CLI to be used in your editor.

## Features

- Auto-format RSX;
- Convert HTML to RSX;
- Convert HTML to Dioxus component;
- Format RSX.

## Commands

### Convert HTML to RSX

Converts a selection of html to valid RSX.

### Convert HTML to Dioxus component

Converts a selection of html to a valid Dioxus component with all SVGs factored out into their own
module.

### Format RSX

Formats RSX macro contents in the current file.

## Building and publishing

Make sure `wasm-pack` are installed and up to date:

```sh
cargo binstall wasm-pack
```

Install dependencies:

```sh
npm install
```

Build and package the extension:

```sh
npm run package
```

Publish the extension:

```sh
npm run publish
```

## Working with Dioxus

To learn more about Dioxus, make sure to check out:

- The [Getting Started][start] guide;
- The [Book][book].

[start]: https://dioxuslabs.com/learn/0.7/getting_started
[book]: https://dioxuslabs.com/learn/0.7/

## Contributing

- Check out the website [section on contributing][contribute].
- Report issues on our [issue tracker][issues].
- [Join the Discord][discord] and ask questions!

<a href="https://github.com/dioxuslabs/dioxus/graphs/contributors">
  <img
    src="https://contrib.rocks/image?repo=dioxuslabs/dioxus&max=30&columns=10"
    alt="Dioxus contributors"
  >
</a>

[contribute]: https://dioxuslabs.com/learn/0.7/beyond/contributing
[issues]: https://github.com/dioxuslabs/dioxus/issues
[discord]: https://discord.gg/XgGxMSkvUM

## License

This project is licensed under the [MIT license][license].

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in
Dioxus by you, shall be licensed as MIT, without any additional terms or conditions.

[license]: https://github.com/DioxusLabs/dioxus/blob/master/LICENSE-MIT
