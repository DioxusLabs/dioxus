// This test checks the CLI optimizes assets correctly without breaking them

use dioxus::prelude::*;

const SOME_IMAGE: Asset = asset!("/images/toasts.png", AssetOptions::image().with_avif());
const SOME_IMAGE_WITH_THE_SAME_URL: Asset =
    asset!("/images/toasts.png", AssetOptions::image().with_jpg());
#[used]
static SOME_IMAGE_WITHOUT_HASH: Asset = asset!(
    "/images/toasts.png",
    AssetOptions::image().with_avif().with_hash_suffix(false)
);
// This asset is unused, but it should still be bundled because it is an external asset
#[used]
static _ASSET: Asset = asset!(
    "/images/toasts.png",
    AssetOptions::builder().with_hash_suffix(false)
);

// Regression coverage for https://github.com/DioxusLabs/dioxus/issues/5512:
// `with_minify(false)` must copy the file byte-for-byte (no IIFE rewrap, no
// ESM transform), and `with_minify(true)` must keep the script as a classic
// script (not ESM) so the `<script>` tag without `type="module"` still runs.
#[used]
static _IIFE_CLASSIC_JS: Asset = asset!(
    "/assets/iife_classic.js",
    AssetOptions::js().with_minify(false).with_static_head(true)
);
#[used]
static _IIFE_MINIFY_JS: Asset = asset!(
    "/assets/iife_minify.js",
    AssetOptions::js().with_minify(true).with_static_head(true)
);
#[used]
static _UMD_MINIFY_JS: Asset = asset!(
    "/assets/umd_minify.js",
    AssetOptions::js().with_minify(true).with_static_head(true)
);
#[used]
static _CJS_CLASSIC_JS: Asset = asset!(
    "/assets/cjs_classic.cjs",
    AssetOptions::js().with_minify(true).with_static_head(true)
);
#[used]
static _SWEETALERT2_JS: Asset = asset!(
    "/assets/sweetalert2.all.min.js",
    AssetOptions::js().with_minify(false).with_static_head(true)
);

// Coverage for `with_module(true)`: the script tag must be emitted as
// `<script type="module">` and minification must preserve module syntax so
// the browser can parse the file as ESM.
#[used]
static _ESM_MODULE_JS: Asset = asset!(
    "/assets/esm_module.js",
    AssetOptions::js().with_module(true).with_static_head(true)
);

// Coverage for module auto-detection: this asset has no `with_module(true)`
// override, but its source contains a top-level `export`, so the CLI's
// has_module_syntax scan must classify it as ESM and emit the script tag with
// `type="module"`. Confirms detection works without explicit user opt-in.
#[used]
static _ESM_AUTO_JS: Asset = asset!(
    "/assets/esm_auto.js",
    AssetOptions::js().with_static_head(true)
);
#[used]
static _ESM_IMPORT_ENTRY_JS: Asset = asset!(
    "/assets/esm_import_entry.js",
    AssetOptions::js().with_static_head(true)
);
#[used]
static _MJS_AUTO_JS: Asset = asset!(
    "/assets/mjs_auto.mjs",
    AssetOptions::js().with_static_head(true)
);

pub fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    // todo: test monaco more....
    // const MONACO_FOLDER: Asset = asset!("/monaco-editor/package/min/vs");
    //     let script = format!("(() => {{
    //     require.config({{ paths: {{ vs: '{MONACO_FOLDER}' }} }});

    //     require(['vs/editor/editor.main'], () => {{
    //         var model = monaco.editor.createModel('fn main() {{\\n\\tprintln!(\\\"hi\\\")\\n}}', 'rust');
    //         var editor = monaco.editor.create(document.getElementById('editor'));
    //         editor.setModel(model);
    //     }})
    // }})()");

    rsx! {
        div {
            id: "editor",
            width: "100vw",
            height: "100vw",
        }
        // // Monaco script
        // script {
        //     src: "{MONACO_FOLDER}/loader.js",
        //     "onload": script
        // }
        img {
            id: "some_image",
            src: "{SOME_IMAGE}"
        }
        img {
            id: "some_image_with_the_same_url",
            src: "{SOME_IMAGE_WITH_THE_SAME_URL}"
        }
        img {
            id: "some_image_without_hash",
            src: "{SOME_IMAGE_WITHOUT_HASH}"
        }
        LoadsAsset {}
    }
}

const JSON: Asset = asset!("/assets/data.json");

#[derive(Debug, Clone, serde::Deserialize)]
struct Data {
    list: Vec<i32>,
}

#[component]
fn LoadsAsset() -> Element {
    let data = use_resource(|| async {
        let bytes = dioxus::asset_resolver::read_asset_bytes(&JSON)
            .await
            .unwrap();
        serde_json::from_slice::<Data>(&bytes).unwrap()
    });
    match data() {
        Some(data) => rsx! {
            div {
                id: "resolved-data",
                "List: {data.list:?}"
            }
        },
        None => rsx! {
            div {
                "Loading..."
            }
        },
    }
}
