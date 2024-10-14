fn main() {
    // If any TS files change, re-run the build script
    lazy_js_bundle::LazyTypeScriptBindings::new()
        .with_watching("./src/ts")
<<<<<<<< HEAD:packages/web/build.rs
        .with_binding("./src/ts/eval.ts", "./src/js/eval.js")
========
        .with_binding("./src/ts/head.ts", "./src/js/head.js")
>>>>>>>> main:packages/html/build.rs
        .run();
}
