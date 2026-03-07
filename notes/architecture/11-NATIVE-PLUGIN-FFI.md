# Native Plugin System Design Document

## Summary

Build a system that lets Rust projects include Swift/Kotlin/C source files via macros, with metadata emitted through the linker. A bundler extracts this metadata and compiles/links the native code into the final app.

We're adding this feature to manganis by leveraging the recently added cbor support for general assets.

## Core Concept

```rust
#[manganis::ffi("/src/android")]
extern "Kotlin" {
    fn do_thing() -> JObject;
}

#[manganis::ffi("/src/ios")]
extern "Swift" {
    fn do_thing() -> NSObject;
}
```

The macro:
1. Emits a new asset type (source folder / source file)
2. Generates the relevant ffi functions
3. The bundler later extracts metadata and compiles the sources

## Why This Approach

| Problem with build.rs | Our solution                                |
| --------------------- | ------------------------------------------- |
| Tied to Cargo         | Linker metadata works with any build system |
| Slows rust-analyzer   | Zero impact on IDE                          |
| Finicky caching       | Bundler handles caching                     |
| Must run before rustc | Bundler runs after rustc                    |

The tradeoff: Rust can't use *outputs* of native compilation (generated headers, etc.). For our use case (runtime FFI via JNI/ObjC), this is fine.

---

## Phase 1: The Macro

### Basic API

```rust
// Annotate an extern block with hardcoded supported languages (swift, kotlin, java, etc)
// Additional props may be passed in as necessary.
#[manganis::ffi("/src/ios", some_prop = "123")]
extern "Swift" {
    // objects that may be declared, using `type` syntax
    pub type SomeSwiftObject;

    // functions associated with said object
    // values passed must be pointer-like or values that can be coerced between languages
    //
    // calling this goes through runtime lookup instead of actually being linked
    // for kotlin, this would be a jni lookup and call
    pub fn do_thing_with_swift_object_swift(this: &SomeSwiftObject) -> Option<u32>;
}

// User can extend the extern types with their own rusty methods
impl SomeSwiftObject {
    pub fn new() -> Self {
        // objc code, constructor maybe?
    }

    // custom extensions
    pub fn custom_wrapper(&mut self) {
        _ = self.do_thing_with_swift_object_swift().unwrap();
    }
}
```

---

## Implementation Checklist

### Macro

- [ ] Add new the manganis types to manganis-core for emitting source folders
- [ ] Come up with more work to make the macro ready

### Bundler

- [ ] Extract ffi blocks from the manganis extractor
- [ ] Parse PluginMeta entries
- [ ] Deduplicate paths
- [ ] Swift compilation
  - [ ] Invoke swiftc
  - [ ] Handle iOS vs macOS targets
  - [ ] Framework/module configuration
- [ ] Kotlin compilation
  - [ ] Invoke kotlinc or gradle
  - [ ] Handle Android-specific setup
  - [ ] DEX generation for Android
- [ ] Link outputs into final binary/bundle
- [ ] Caching (hash sources, skip if unchanged)

### Integration with dx (your bundler)

- [ ] Hook into post-rustc phase
- [ ] Call plugin extraction
- [ ] Call compilation pipeline
- [ ] Include outputs in app bundle

---

## Platform-Specific Notes

### Swift Compilation

```bash
# iOS
swiftc -target arm64-apple-ios15.0 \
  -sdk /path/to/iPhoneOS.sdk \
  -emit-library \
  -o libplugins.a \
  LocationManager.swift

# macOS
swiftc -emit-library \
  -o libplugins.dylib \
  LocationManager.swift
```

### Kotlin Compilation

For Android, you need to produce DEX:

```bash
# Compile to class files
kotlinc Geolocator.kt -include-runtime -d classes/

# Convert to DEX
d8 classes/*.class --output dex/
```

Or use Gradle if dependencies are needed.

### C Compilation

```bash
# Android
$NDK/toolchains/llvm/prebuilt/*/bin/clang \
  --target=aarch64-linux-android21 \
  -c fast_math.c -o fast_math.o

# iOS
clang -target arm64-apple-ios15.0 \
  -isysroot /path/to/iPhoneOS.sdk \
  -c fast_math.c -o fast_math.o
```

---

## Testing Strategy

1. **Macro tests**: Verify correct link_section output using `trybuild` or manual inspection
2. **Extraction tests**: Create test binaries, verify metadata extraction works
3. **Compilation tests**: Verify Swift/Kotlin/C compilation produces valid artifacts
4. **Integration tests**: End-to-end build of sample app with plugins
