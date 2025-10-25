# Geolocation Demo

A demonstration of the `dioxus-mobile-geolocation` crate with a beautiful UI.

## Features

- 📍 Get current location from Android/iOS devices
- 🗺️ View location on Google Maps
- ✨ Beautiful gradient UI with responsive design
- 🔒 Automatic permission management via linker symbols
- 🤖 Android support via Kotlin shim
- 🍎 iOS support via Swift shim

## Prerequisites

### Android
- Android SDK with API level 24+
- Android emulator or physical device

### iOS
- Xcode 14+ with iOS SDK
- iOS Simulator or physical device

## Running the Example

### Android

```bash
# Build for Android
dx build --platform android

# Run on connected device/emulator
dx run --device
```

### iOS

```bash
# Build for iOS
dx build --platform ios

# Run on simulator
dx run --device
```

## How It Works

1. **Permissions**: The `dioxus-mobile-geolocation` crate embeds location permissions as linker symbols
2. **CLI Injection**: The Dioxus CLI scans the binary and automatically injects permissions into `AndroidManifest.xml` or `Info.plist`
3. **Platform Shims**: Kotlin (Android) and Swift (iOS) shims are compiled during `cargo build`
4. **Runtime**: The app requests location permissions at runtime before accessing location

## Troubleshooting

### No location available

- Make sure location services are enabled on your device
- Grant location permission when prompted
- Try opening Google Maps first to get an initial location fix
- For Android simulator, use the extended controls to set a mock location

### Build errors

- Ensure Android SDK is installed and `ANDROID_HOME` is set
- For iOS, ensure Xcode command line tools are installed
- Run `cargo clean` and rebuild if issues persist

## Screenshots

The app features:
- Gradient header with platform indicator
- Status card showing location state
- Coordinate display with precise lat/lon
- Google Maps link for visualization
- Info section explaining how it works

Built with Dioxus 🦀

