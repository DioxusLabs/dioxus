# CLI Integration for Android JAR/AAR

## Current Status

The `dioxus-mobile-geolocation` crate builds its Kotlin shim as an AAR file during `cargo build`. However, the Dioxus CLI currently doesn't automatically include external AAR/JAR files from build scripts into the Android app.

## Manual Workaround

After building your app, manually copy the AAR:

```bash
# After running dx serve --android
cp target/android-dev/deps/build/dioxus-mobile-geolocation-*/out/geolocation-shim.aar \
   target/android-dev/app/libs/
```

Or use this helper script:

```bash
#!/bin/bash
# Copy geolocation AAR to Android libs

AAR=$(find target/android-dev/deps/build -name "geolocation-shim.aar" | head -1)
LIBS_DIR="target/android-dev/app/libs"

if [ -f "$AAR" ]; then
    mkdir -p "$LIBS_DIR"
    cp "$AAR" "$LIBS_DIR/"
    echo "✅ Copied AAR to $LIBS_DIR"
else
    echo "❌ AAR not found"
fi
```

## Future Improvement

The CLI should be enhanced to:
1. Scan `$OUT_DIR` directories for `*.aar` and `*.jar` files
2. Automatically copy them to `android/app/libs/`
3. Ensure the Gradle build includes them

## Current Build Flow

1. `cargo build` compiles Rust and runs `build.rs`
2. `build.rs` invokes Gradle to build Kotlin shim
3. AAR is produced in `android-shim/build/outputs/aar/`
4. AAR is copied to `$OUT_DIR/geolocation-shim.aar`
5. ✅ **Manual step**: Copy AAR to CLI's `android/app/libs/`
6. CLI generates Android project
7. CLI runs Gradle to build APK

Step 5 is currently manual and should be automated by the CLI.

