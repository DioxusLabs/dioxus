# Publishing

Congrats! You've made your first Dioxus app that actually does some pretty cool stuff. This app uses your operating system's WebView library, so it's portable to be distributed for other platforms.

In this section, we'll cover how to bundle your app for macOS, Windows, and Linux.



## Install `cargo-bundle`


The first thing we'll do is install [`cargo-bundle`](https://github.com/burtonageo/cargo-bundle). This extension to cargo will make it very easy to package our app for the various platforms.

According to the `cargo-bundle` github page, 



*"cargo-bundle is a tool used to generate installers or app bundles for GUI  executables built with cargo. It can create .app bundles for Mac OS X and iOS, .deb packages for Linux, and .msi installers for Windows (note however that iOS and Windows support is still experimental). Support for creating .rpm packages (for Linux) and .apk packages (for Android) is still pending."*


To install, simply run


`cargo install cargo-bundle`

## Setting up your project


To get a project setup for bundling, we need to add some flags to our `Cargo.toml` file. 


```toml
[package]
name = "example"
# ...other fields...

[package.metadata.bundle]
name = "DogSearch"
identifier = "com.dogs.dogsearch"
version = "1.0.0"
copyright = "Copyright (c) Jane Doe 2016. All rights reserved."
category = "Developer Tool"
short_description = "Easily search for Dog photos"
long_description = """
This app makes it quick and easy to browse photos of dogs from over 200 bree
"""
```


## Building

Following cargo-bundle's instructions, we simply `cargo-bundle --release` to produce a final app with all the optimizations and assets builtin.

Once you've ran `cargo-bundle --release`, your app should be accessible in

`target/release/bundle/<platform>/`.

For example, a macOS app would look like this:

![Published App](../images/publish.png)

Nice! And it's only 4.8 Mb - extremely lean!! Because Dioxus leverages your platform's native WebView, Dioxus apps are extremely memory efficient and won't waste your battery.

> Note: not all CSS works the same on all platforms. Make sure to view your app's CSS on each platform - or web browser (Firefox, Chrome, Safari) before publishing.

