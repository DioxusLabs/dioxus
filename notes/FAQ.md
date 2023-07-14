
### Aren't VDOMs just pure overhead? Why not something like Solid or Svelte?
Remember: Dioxus is a library - not a compiler like Svelte. Plus, the inner VirtualDOM allows Dioxus to easily port into different runtimes, support SSR, and run remotely in the cloud. VDOMs tend to more ergonomic to work with and feel roughly like natural Rust code. The overhead of Dioxus is **extraordinarily** minimal... sure, there may be some overhead but on an order of magnitude lower than the time required to actually update the page.


### Isn't the overhead for interacting with the DOM from Wasm too much?
The overhead layer between Wasm and JS APIs is extremely poorly understood. Rust web benchmarks typically suffer from differences in how Rust and JS cache strings. In Dioxus, we solve most of these issues and our JS Framework Benchmark actually beats the Wasm Bindgen benchmark in many cases. Compared to a "pure vanilla JS" solution, Dioxus adds less than 5% of overhead and takes advantage of batched DOM manipulation.

### Aren't Wasm binaries too huge to deploy in production?
Wasm binary sizes are another poorly understood characteristic of Rust web apps. 50kb of Wasm and 50kb of JS are _not_ made equally. In JS, the code must be downloaded _first_ and _then_ JIT-ted. Just-in-time compiling 50kb of JavaScript takes a while which is why 50kb of JavaScript sounds like a lot! However, with Wasm, the code is downloaded and JIT-ted _simultaneously_ through the magic of streaming compilation. By the time the 50kb of Rust is finished downloading, it is already ready to go. Again, Dioxus beats out many benchmarks with time-to-interactivity.

For reference, Dioxus `hello-world` clocks in at around 70kb gzip or 60kb brotli, and Dioxus supports SSR.

### Why hooks? Why not MVC, classes, traits, messages, etc?
There are plenty Rust Elm-like frameworks in the world - we were not interested in making another! Instead, we borrowed hooks from React. JS and Rust share many structural similarities, so if you're comfortable with React, then you'll be plenty comfortable with Dioxus.

### Why a custom DSL? Why not just pure function calls?
The `RSX` DSL is _barely_ a DSL. Rustaceans will find the DSL very similar to simply assembling nested structs, but without the syntactical overhead of "Default" everywhere or having to jump through hoops with the builder pattern. Between RSX, HTML, the Raw Factory API, and the NodeBuilder syntax, there's plenty of options to choose from.

### What are the build times like? Why on earth would I choose Rust instead of JS/TS/Elm?
dx builds as roughly as fast as a complex WebPack-TypeScript site. Compile times will be slower than an equivalent TypeScript site, but not unbearably slow. The Wasm compiler backend for Rust is very fast. Iterating on small components is basically instant and larger apps takes a few seconds. In practice, the compiler guarantees of Rust balance out the rebuild times.

### What about Yew/Seed/Sycamore/Dominator/Dodrio/Percy?
- Yew and Seed use an Elm-like pattern and don't support SSR or any alternate rendering platforms
- Sycamore and Dominator are more like SolidJS/Svelte, requiring no VDOM but has less naturally-Rusty state management
- Percy isn't quite mature yet
- Dodrio is the spiritual predecessor of Dioxus, but is currently an archived research project without the batteries of Dioxus

### How do the mobile and desktop renderers work? Is it Electron?
Currently, Dioxus uses your device's native WebView library to draw the page. None of your app code is actually running in the WebView thread, so you can access system resources instead of having to go through something like NodeJS. This means your app will use Safari on macOS/iOS, Edge (Chromium) on Windows, and whatever is the default Web Browser for Linux and Android. Because your code is compiled and running natively, performance is not a problem. You will have to use the various "Escape Hatches" to use browser-native APIs (like WebGL) and work around visual differences in how Safari and Chrome render the page.

In the future, we are interested in using Webrenderer to provide a fully native renderer without having to go through the system WebView library. In practice, Dioxus mobile and desktop are great for CRUD-style apps, but the ergonomic cross-platform APIs (GPS, Camera, etc) are not there yet.
