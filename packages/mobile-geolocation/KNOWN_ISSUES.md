# Known Issues

## iOS Swift Shim

### Issue
The iOS Swift shim is currently not building correctly. The `swift build` command in `build.rs` fails with unclear errors.

### Current Status
- ✅ Kotlin shim for Android works correctly
- ❌ Swift shim for iOS needs fixing
- The crate will still compile, but iOS functionality won't work

### Workaround
For development/testing, you can:
1. Focus on Android testing (which works)
2. Manually build the Swift shim separately
3. Temporarily disable iOS feature: `default-features = false, features = ["android-kotlin", "location-coarse"]`

### Error Messages
```
warning: Swift build failed with status: exit status: 1
warning: Continuing without Swift shim (iOS functionality will not work)
error: could not find native static library `GeolocationShim`
```

### Future Fix
The Swift build process needs to be improved. Possible solutions:
1. Use `xcodebuild` instead of `swift build`
2. Create a proper Xcode project instead of Swift Package
3. Simplify the Swift shim compilation process

## Impact
- Android development and testing: ✅ Works
- iOS development and testing: ❌ Blocked by Swift shim issue
- Production use: Android ready, iOS needs fixing

