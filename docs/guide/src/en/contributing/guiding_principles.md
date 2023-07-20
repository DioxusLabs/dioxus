# Overall Goals

This document outlines some of the overall goals for Dioxus. These goals are not set in stone, but they represent general guidelines for the project.

The goal of Dioxus is to make it easy to build **cross-platform applications that scale**.

## Cross-Platform

Dioxus is designed to be cross-platform by default. This means that it should be easy to build applications that run on the web, desktop, and mobile. However, Dioxus should also be flexible enough to allow users to opt into platform-specific features when needed. The `use_eval` is one example of this. By default, Dioxus does not assume that the platform supports JavaScript, but it does provide a hook that allows users to opt into JavaScript when needed.

## Performance

As Dioxus applications grow, they should remain relatively performant without the need for manual optimizations. There will be cases where manual optimizations are needed, but Dioxus should try to make these cases as rare as possible.

One of the benefits of the core architecture of Dioxus is that it delivers reasonable performance even when components are rerendered often. It is based on a Virtual Dom which performs diffing which should prevent unnecessary re-renders even when large parts of the component tree are rerun. On top of this, Dioxus groups static parts of the RSX tree together to skip diffing them entirely.

## Type Safety

As teams grow, the Type safety of Rust is a huge advantage. Dioxus should leverage this advantage to make it easy to build applications with large teams.

To take full advantage of Rust's type system, Dioxus should try to avoid exposing public `Any` types and string-ly typed APIs where possible.

## Developer Experience

Dioxus should be easy to learn and ergonomic to use.

- The API of Dioxus attempts to remain close to React's API where possible. This makes it easier for people to learn Dioxus if they already know React

- We can avoid the tradeoff between simplicity and flexibility by providing multiple layers of API: One for the very common use case, one for low-level control

  - Hooks: the hooks crate has the most common use cases, but `cx.hook` provides a way to access the underlying persistent reference if needed.
  - The builder pattern in platform Configs: The builder pattern is used to default to the most common use case, but users can change the defaults if needed.

- Documentation:
  - All public APIs should have rust documentation
  - Examples should be provided for all public features. These examples both serve as documentation and testing. They are checked by CI to ensure that they continue to compile
  - The most common workflows should be documented in the guide
