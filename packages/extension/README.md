# Dioxus VSCode Extension

![Dioxus Splash](https://github.com/DioxusLabs/dioxus/raw/main/notes/dioxus_splash_8.avif)

This extension wraps functionality in Dioxus CLI to be used in your editor!

## Features:

- Auto-format RSX
- Convert HTML to RSX
- Convert HTML to Dioxus Component
- Format RSX

## Current commands:

### Convert HTML to RSX
Converts a selection of html to valid rsx.

### Convert HTML to Dioxus Component

Converts a selection of html to a valid Dioxus component with all SVGs factored out into their own module.

### Format RSX

Formats the current file as RSX.

## Building this extension from source
- make sure wasm-bindgen is installed to current version (cargo binstall wasm-bindgen-cli)
- run `npm install`
- run `npm run vsix`

# Working with Dioxus:

This overview provides a brief introduction to Dioxus. For a more in-depth guide, make sure to check out:

- [Getting Started](https://dioxuslabs.com/learn/0.7/getting_started)
- [Book (0.6)](https://dioxuslabs.com/learn/0.7/)

## Contributing
- Check out the website [section on contributing](https://dioxuslabs.com/learn/0.7/beyond/contributing).
- Report issues on our [issue tracker](https://github.com/dioxuslabs/dioxus/issues).
- [Join](https://discord.gg/XgGxMSkvUM) the discord and ask questions!


<a href="https://github.com/dioxuslabs/dioxus/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=dioxuslabs/dioxus&max=30&columns=10" />
</a>

## License
This project is licensed under the [MIT license].

[mit license]: https://github.com/DioxusLabs/dioxus/blob/master/LICENSE-MIT

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Dioxus by you, shall be licensed as MIT, without any additional
terms or conditions.
