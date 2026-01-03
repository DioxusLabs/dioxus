# platform-bridge-macro

Procedural macros for declaring Android and iOS/macOS plugin metadata that the Dioxus CLI collects
from linker symbols.

## Overview

This crate exposes the `#[native_plugin]` macro which allows include source folders from other languages into your Rust code.

```rust
#[native_plugin("/src/ios/")]
extern "Kotlin" {
    //
}

#[native_plugin("/src/ios/")]
extern "Swift" {
    //
}

#[native_plugin("/src/ts/")]
extern "Typescript" {
    //
}
```

