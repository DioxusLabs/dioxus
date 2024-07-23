fn main() {
    // If any TS files change, re-run the build script
    lazy_js_bundle::LazyTypeScriptBindings::new()
        .with_watching("./src/ts")
        .with_binding("./src/ts/common.ts", "./src/js/common.js")
        .with_binding("./src/ts/native.ts", "./src/js/native.js")
        .with_binding("./src/ts/core.ts", "./src/js/core.js")
        .with_binding("./src/ts/hydrate.ts", "./src/js/hydrate.js")
        .with_binding("./src/ts/patch_console.ts", "./src/js/patch_console.js")
        .with_binding(
            "./src/ts/initialize_streaming.ts",
            "./src/js/initialize_streaming.js",
        )
        .run();
}
