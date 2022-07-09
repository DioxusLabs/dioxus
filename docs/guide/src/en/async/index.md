# Working with Async

Often, apps need to interact with file systems, network interfaces, hardware, or timers. This chapter provides an overview of using async code in Dioxus.

## The Runtime

By default, Dioxus-Desktop ships with the `Tokio` runtime and automatically sets everything up for you. This is currently not configurable, though it would be easy to write an integration for Dioxus desktop that uses a different asynchronous runtime.

Dioxus is not currently thread-safe, so any async code you write does *not* need to be `Send/Sync`. That means that you can use non-thread-safe structures like `Cell`, `Rc`, and `RefCell`.
