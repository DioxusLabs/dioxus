# Dioxus Mobile demo

## How this project was generated

Right now, Dioxus supports mobile targets including iOS and Android. However, our tooling is not mature enough to include the build commands directly.

This project was generated using [cargo-mobile2](https://github.com/tauri-apps/cargo-mobile2). We have yet to integrate this generation into the Dioxus-CLI. The open issue for this is [#1157](https://github.com/DioxusLabs/dioxus/issues/1157).

## Running on iOS

First, you'll want to make sure you have the appropriate iOS targets installed.

The two targets you'll use the most are:

- `aarch64-apple-ios-sim`
- `aarch64-apple-ios`

These can be added using
- `rustup target add aarch64-apple-ios-sim`
- `rustup target add aarch64-apple-ios`

From there, you'll want to get a build of the crate using whichever platform you're targeting (simulator or actual hardware). For now, we'll just stick with the simulator:
- `cargo build --target aarch64-apple-ios-sim`

Then, you'll want to open XCode. This might take awhile if you've never opened XCode before. The command you want to use is:
- `cargo apple open`

This will open XCode with this particular project.

From there, just click the "play" button with the right target and the app should be running!

![ios_demo](ios_demo.png)

Note that clicking play doesn't cause a new build, so you'll need to keep rebuilding the app between changes. The tooling here is very young, so please be patient. If you want to contribute to make things easier, please do! We'll be happy to help.


## Running on Android

Again, we want to make sure we have the right targets installed.

The common targets here are
- aarch64-linux-android
- armv7-linux-androideabi
- i686-linux-android
- x86_64-linux-android

