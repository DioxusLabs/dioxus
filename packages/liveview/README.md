# Livehost

Stream events from a server to a client.

This crate provides drivers for Actix, Warp, and Tide to run coupled frontend and backend. 

This comes in the form of two approaches:

- tight coupling: frontend and backend are locked together
- loose coupling: hooks subscribe a component to a server using the suspense mechanism

Tight coupling is basically an implmentation of loose coupling where **all** events move through the backend connection. This coupling option has higher latency but is very simple to deploy. We use this approach for dioxus-webview where latency is minimal (hosted locally) and we want builds to be simple - no need to manually bundle a custom frontend because everything is server rendered.

