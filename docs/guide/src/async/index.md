# Working with Async

Not all apps you'll build can be self-contained with synchronous code. You'll often need to interact with file systems, network interfaces, hardware, or timers.

So far, we've only talked about building apps with synchronous code, so this chapter will focus integrating asynchronous code into your app.


## The Runtime

By default, Dioxus-Desktop ships with the `Tokio` runtime and automatically sets everything up for you.



## Send/Sync
Writing apps that deal with Send/Sync can be frustrating at times. Under the hood, Dioxus is not currently thread-safe, so any async code you write does *not* need to be `Send/Sync`. That means Cell/Rc/RefCell are all fair game.



All async code in your app is polled on a `LocalSet`, so any async code we w


> This section is currently under construction! 🏗
