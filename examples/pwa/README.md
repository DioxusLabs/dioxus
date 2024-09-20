# Dioxus PWA example

This is a basic example of a progressive web app (PWA) using Dioxus and Dioxus CLI.
Currently PWA functionality requires the use of a service worker and manifest file, so this isn't 100% Rust yet.

It is also very much usable as a template for your projects, if you're aiming to create a PWA.

## Try the example

Make sure you have Dioxus CLI installed (if you're unsure, run `cargo install dioxus-cli --locked`).

You can run `dx serve` in this directory to start the web server locally, or run
`dx build --release` to build the project so you can deploy it on a separate web-server.

## Project Structure

```
├── Cargo.toml
├── Dioxus.toml
├── index.html // Custom HTML is needed for this, to load the SW and manifest.
├── LICENSE
├── public
│   ├── favicon.ico
│   ├── logo_192.png
│   ├── logo_512.png
│   ├── manifest.json // The manifest file - edit this as you need to.
│   └── sw.js // The service worker - you must edit this for actual projects.
├── README.md
└── src
    └── main.rs
```

## Resources

If you're just getting started with PWAs, here are some useful resources:

- [PWABuilder docs](https://docs.pwabuilder.com/#/)
- [MDN article on PWAs](https://developer.mozilla.org/en-US/docs/Web/Progressive_web_apps)

For service worker scripting (in JavaScript):

- [Service worker guide from PWABuilder](https://docs.pwabuilder.com/#/home/sw-intro)
- [Service worker examples, also from PWABuilder](https://github.com/pwa-builder/pwabuilder-serviceworkers)

If you want to stay as close to 100% Rust as possible, you can try using [wasi-worker](https://github.com/dunnock/wasi-worker) to replace the JS service worker file. The JSON manifest will still be required though.
