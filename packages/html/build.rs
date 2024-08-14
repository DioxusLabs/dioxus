fn main() {
    // If any TS files change, re-run the build script
    lazy_js_bundle::LazyTypeScriptBindings::new()
        .with_watching("./src/ts")
        .with_binding("./src/ts/eval.ts", "./src/js/eval.js")
        .with_binding("./src/ts/native_eval.ts", "./src/js/native_eval.js")
        .with_binding("./src/ts/head.ts", "./src/js/head.js")
        .run();
}
