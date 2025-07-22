// This test checks the CLI optimizes assets correctly without breaking them

use dioxus::prelude::*;

const MONACO_FOLDER: Asset = asset!("/monaco-editor/package/min/vs");
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

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let script = format!("(() => {{
    require.config({{ paths: {{ vs: '{MONACO_FOLDER}' }} }});

    require(['vs/editor/editor.main'], () => {{
        var model = monaco.editor.createModel('fn main() {{\\n\\tprintln!(\\\"hi\\\")\\n}}', 'rust');
        var editor = monaco.editor.create(document.getElementById('editor'));
        editor.setModel(model);
    }})
}})()");
    rsx! {
        div {
            id: "editor",
            width: "100vw",
            height: "100vw",
        }
        // Monaco script
        script {
            src: "{MONACO_FOLDER}/loader.js",
            "onload": script
        }
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
    }
}
