# dioxus webview arch


dioxus webview represents a usecase of dioxus to stream edits from one dom to another. In this particular case, we stream edits from a native process into the webview's runtime. WebView is really portable, so we can leverage the UI of iOS, Android, and all desktop platforms while also maintaining direct access to platform-specific APIs. It's the best of both worlds!


For this to work properly, we need to ship a universal client that receives Dioxus edits from a remote process. To make our lives (as developers) a bit easier, the client code is currently implemented in JS (subject to change!).

The provider (the thing that talks to the client on the other side of the network) is still written in Rust, and is supplied through the dioxus-webview crate. The client is much more "dumb" than the provider - the provider must handle NodeID generation, flushing changes, priorities, managing events, and more.


## update; wry!

Wry is an effort by the Tauri team to make fully native 
