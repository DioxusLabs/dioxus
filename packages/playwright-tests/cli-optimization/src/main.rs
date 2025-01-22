// This test checks the CLI optimizes assets correctly without breaking them

use dioxus::prelude::*;

const MONACO_FOLDER: Asset = asset!("/monaco-editor-0.52.2/package/min/vs");
const SOME_IMAGE: Asset = asset!("/images/toasts.png", ImageAssetOptions::new().with_avif());

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
            src: "{SOME_IMAGE}"
        }
    }
}
