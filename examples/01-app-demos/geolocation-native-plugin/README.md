# Geolocation demo

A minimal Dioxus application that implements a native plugin.

The plugin demonstrated here makes it possible to access the user's geolocation. It does a few things:

- Inspect and request location permissions using the native Android/iOS dialogs.
- Configure one-shot position requests (high-accuracy toggle + maximum cached age).
- Inspect the last reported coordinates, accuracy, altitude, heading, and speed.

The example shares the same metadata pipeline as any plugin crate: the native Gradle/Swift
artifacts are embedded via linker symbols and bundled automatically by `dx`.

## Running the example

```bash
# Inside the repository root
dx serve --project examples/01-app-demos/geolocation --platform mobile
```

For Android/iOS you’ll need the respective toolchains installed (Android SDK/NDK, Xcode) so the
geolocation crate’s `build.rs` can build the native modules. The UI also works on desktop/web,
but location calls will return an error because the plugin only supports mobile targets—those
errors are shown inline in the demo.

## Things to try

1. Tap **Check permissions** to see the current OS state (granted/denied/prompt).
2. Tap **Request permissions** to trigger the native dialog from within the app.
3. Toggle *High accuracy* and set a *Max cached age* before requesting the current position.
4. Observe the coordinate grid update whenever a new reading arrives, or the error banner if the
   operation fails (e.g., permissions denied or running on an unsupported platform).
