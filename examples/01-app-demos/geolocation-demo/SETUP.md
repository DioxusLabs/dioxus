# Android Development Setup for Geolocation Demo

## Prerequisites

1. **Android Studio** (includes Android SDK)
2. **Android NDK** (for Rust compilation)

## Setup Steps

### 1. Install Android Studio

Download from: https://developer.android.com/studio

### 2. Install Android NDK

1. Open Android Studio
2. Go to Tools > SDK Manager
3. Click on "SDK Tools" tab
4. Check "NDK (Side by side)" and "CMake"
5. Click "Apply" to install

### 3. Set Environment Variables

**Quick setup (for this session):**
```bash
cd examples/01-app-demos/geolocation-demo
source setup-android.sh
```

**Permanent setup (add to `~/.zshrc`):**
```bash
# Android SDK
export ANDROID_HOME=$HOME/Library/Android/sdk

# Android NDK (use the version you have installed)
export ANDROID_NDK_HOME=$HOME/Library/Android/sdk/ndk/27.0.12077973

# Add SDK tools to PATH
export PATH=$PATH:$ANDROID_HOME/platform-tools
export PATH=$PATH:$ANDROID_HOME/tools
export PATH=$PATH:$ANDROID_HOME/cmdline-tools/latest/bin
```

Reload your shell:
```bash
source ~/.zshrc
```

### 4. Install Rust Android Target

```bash
rustup target add aarch64-linux-android
```

### 5. Verify Installation

```bash
# Check Android SDK
$ANDROID_HOME/platform-tools/adb version

# Check NDK (if using specific version)
ls $ANDROID_HOME/ndk/

# Check Rust targets
rustup target list --installed | grep android
```

### 6. Create Android Virtual Device (AVD)

1. Open Android Studio
2. Go to Tools > Device Manager
3. Click "Create Device"
4. Select a device (e.g., Pixel 6)
5. Select a system image (API 34 recommended)
6. Click "Finish"

### 7. Start Emulator

```bash
# List available AVDs
emulator -list-avds

# Start an emulator
emulator -avd Pixel_6_API_34 &

# Or use Android Studio's Device Manager to start it
```

### 8. Enable Location on Emulator

Once emulator is running:
1. Open Settings
2. Go to Location
3. Turn on "Use location"
4. Set to "High accuracy" mode

### 9. Set Mock Location (Optional)

Open Extended Controls (`...` on sidebar):
1. Go to Location tab
2. Enter coordinates (e.g., Mountain View):
   - Latitude: `37.421998`
   - Longitude: `-122.084`
3. Click "Set Location"

### 10. Run the Demo

```bash
cd examples/01-app-demos/geolocation-demo

# Build and run
dx serve --android

# Or build and install manually
dx build --platform android
dx run --device
```

## Troubleshooting

### "Android not installed properly"

Make sure `ANDROID_NDK_HOME` is set correctly:
```bash
export ANDROID_NDK_HOME=$HOME/Library/Android/sdk/ndk
```

### "dx and dioxus versions are incompatible"

Make sure you're using `dx` version 0.7.0-rc.3:
```bash
cargo install --git https://github.com/DioxusLabs/dioxus --tag v0.7.0-rc.3 dioxus-cli
```

### "Device not found"

Make sure emulator is running:
```bash
adb devices
```

If empty, start the emulator or connect a physical device.

### Build fails

Try cleaning and rebuilding:
```bash
cargo clean
dx build --platform android
```

## Alternative: Use Physical Device

1. Enable Developer Options on your Android device
2. Enable USB Debugging
3. Connect via USB
4. Accept the debugging prompt on device
5. Run `adb devices` to verify connection
6. Run `dx serve --android`

