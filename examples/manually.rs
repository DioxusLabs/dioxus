use dioxus_core::*;

fn main() {
    use DomEdit::*;

    // .. should result in an "invalid node tree"
    let edits = vec![
        CreateElement { tag: "div", id: 0 },
        // CreatePlaceholder { id: 1 },
        CreateElement { tag: "h1", id: 2 },
        CreateTextNode {
            text: "hello world",
            id: 3,
        },
        AppendChildren { many: 1 },
        AppendChildren { many: 1 },
        AppendChildren { many: 1 },
        // ReplaceWith { many: 1 },
    ];
    dioxus_desktop::WebviewRenderer::run_with_edits(App, (), |c| c, Some(edits)).expect("failed");
}

const App: FC<()> = |cx, props| todo!();
