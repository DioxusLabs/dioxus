# Antipatterns

This example shows what not to do and provides a reason why a given pattern is considered an "AntiPattern". Most anti-patterns are considered wrong due performance reasons, or for harming code re-usability.

# Unnecessarily Nested Fragments

Fragments don't mount a physical element to the DOM immediately, so Dioxus must recurse into its children to find a physical DOM node. This process is called "normalization". This means that deeply nested fragments make Dioxus perform unnecessary work. Prefer one or two levels of fragments / nested components until presenting a true DOM element.

Only Component and Fragment nodes are susceptible to this issue. Dioxus mitigates this with components by providing an API for registering shared state without the Context Provider pattern.

```rust
{{#include ../../examples/anti_patterns.rs:nested_fragments}}
```

# Libraries with Unnecessary Features Enabled

When publishing your custom hook or component, we highly advise using only the core feature on the `dioxus` crate. This makes your crate compile faster, makes it more stable, and avoids bringing in incompatible libraries that might make it not compile on unsupported platforms.


❌ Don't include unnecessary dependencies in libraries:
```toml
dioxus = { version = "...", features = ["web", "desktop", "full"]}
```

✅ Only add the features you need:
```toml
dioxus = { version = "...", features = "core"}
```

# Incorrect Iterator Keys

As described in the conditional rendering chapter, list items must have unique keys that are associated with the same items across renders. This helps Dioxus associate state with the contained components, and ensures good diffing performance. Do not omit keys, unless you know that the list is static and will never change.

```rust
{{#include ../../examples/anti_patterns.rs:iter_keys}}
```