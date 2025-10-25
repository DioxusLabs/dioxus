# Testing Geolocation Demo on Android Simulator

## Quick Start

```bash
# Navigate to the example directory
cd examples/01-app-demos/geolocation-demo

# Build for Android
dx build --platform android

# Run on Android emulator
dx run --device
```

## Step-by-Step Testing Guide

### 1. Start Android Emulator

```bash
# List available emulators
emulator -list-avds

# Start an emulator (replace with your AVD name)
emulator -avd Pixel_6_API_34
```

**Or use Android Studio:**
- Open Android Studio
- Go to Tools > Device Manager
- Start an emulator

### 2. Enable Location on Emulator

The Android emulator needs location services enabled:

1. Open Settings on the emulator
2. Go to Location
3. Turn on "Use location"
4. Set it to "High accuracy" mode

### 3. Set Mock Location (Optional)

To test with a specific location:

1. Open Extended Controls in emulator (click `...` on sidebar)
2. Go to Location tab
3. Enter coordinates (e.g., Mountain View, CA):
   - Latitude: `37.421998`
   - Longitude: `-122.084`
4. Click "Set Location"

Or use Google Maps app:
1. Open Google Maps on emulator
2. Let it get your location
3. This creates a cached location that our app can read

### 4. Build and Run

```bash
# Build for Android
dx build --platform android

# Install and run on emulator
dx run --device
```

### 5. Grant Permissions

When the app launches:
1. Click "📍 Get My Location" button
2. Grant location permission when prompted
3. The app will display your coordinates

## Expected Behavior

✅ **Success**: App shows your location coordinates  
✅ **With Mock Location**: App shows the coordinates you set  
❌ **No Permission**: App shows "No location available"  
❌ **Services Disabled**: App shows "No location available"  

## Troubleshooting

### "Class not found" error

The Kotlin shim AAR is not included. Make sure you're building with `dx build`, not just `cargo build`.

### Permission denied

- Make sure you grant the permission when prompted
- Check app permissions in Settings > Apps > Geolocation Demo > Permissions

### No location available

- Enable location services in device settings
- Set a mock location in emulator
- Open Google Maps first to get initial location fix

### Build fails

```bash
# Clean and rebuild
cd ../../../
cargo clean
cd examples/01-app-demos/geolocation-demo
dx build --platform android
```

## iOS Testing

For iOS testing on simulator:

```bash
# Build for iOS
dx build --platform ios

# Run on iOS simulator
dx run --device
```

Note: iOS simulator doesn't have a real GPS, so you'll need to set a mock location via Simulator menu > Features > Location > Custom Location.

## Verification

After running successfully, you should see:
- ✅ Status message: "Location retrieved successfully!"
- 📍 Latitude and Longitude displayed
- 🗺️ Google Maps link to view on map

## Debug Tips

Enable verbose logging:
```bash
RUST_LOG=debug dx run --device
```

Check logs:
```bash
# Android
adb logcat | grep -i geolocation

# iOS
# View console output in Xcode
```

