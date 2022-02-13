# JS Interpreter

After diffing old and new trees, the Dioxus VirtualDom produces patches that are used to modify the existing Dom. We can send these patches anywhere - including targets without WASM support.

In renderers with support for JavaScript, we use the interpreter from this repository - written in TypeScript - to patch the Dom. This lets us circumvent any overhead on the Rust <-> Dom boundary and keep consistency in our interpreter implementation in web/webview targets.

For now - both Dioxus Web and Dioxus Desktop (webview) use the same interpreter code with tweaks.
