# Antipatterns

This example shows what not to do and provides a reason why a given pattern is considered an "AntiPattern". Most anti-patterns are considered wrong for performance or code re-usability reasons.

## Unnecessarily Nested Fragments

Fragments don't mount a physical element to the DOM immediately, so Dioxus must recurse into its children to find a physical DOM node. This process is called "normalization". This means that deeply nested fragments make Dioxus perform unnecessary work. Prefer one or two levels of fragments / nested components until presenting a true DOM element.

Only Component and Fragment nodes are susceptible to this issue. Dioxus mitigates this with components by providing an API for registering shared state without the Context Provider pattern.

```rust
{{#include ../../../examples/anti_patterns.rs:nested_fragments}}
```

## Incorrect Iterator Keys

As described in the [dynamic rendering chapter](../interactivity/dynamic_rendering.md#the-key-attribute), list items must have unique keys that are associated with the same items across renders. This helps Dioxus associate state with the contained components and ensures good diffing performance. Do not omit keys, unless you know that the list will never change.

```rust
{{#include ../../../examples/anti_patterns.rs:iter_keys}}
```

## Avoid Interior Mutability in Props

While it is technically acceptable to have a `Mutex` or a `RwLock` in the props, they will be difficult to use.

Suppose you have a struct `User` containing the field `username: String`. If you pass a `Mutex<User>` prop to a `UserComponent` component, that component may wish to pass the username as a `&str` prop to a child component. However, it cannot pass that borrowed field down, since it only would live as long as the `Mutex`'s lock, which belongs to the `UserComponent` function. Therefore, the component will be forced to clone the `username` field.

## Avoid Updating State During Render

Every time you update the state, Dioxus needs to re-render the component – this is inefficient! Consider refactoring your code to avoid this.

Also, if you unconditionally update the state during render, it will be re-rendered in an infinite loop.