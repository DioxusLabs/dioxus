// Edge cases that markerless hydration must handle without producing
// hydration comments or `data-node-hydration` attributes.

use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    use_hook(dioxus::fullstack::commit_initial_chunk);

    rsx! {
        DangerousInnerHtml {}
        AdjacentDynamicTexts {}
        LongEmptyRuns {}
        Utf16Text {}
        EmptyMiddleOfRun {}
        ComponentInTextRun {}
        EmptyPlaceholderToContent {}
        SeparatedEmptyDynamicSlots {}
        PlaceholderAsTrailingSibling {}
        NestedTrailingPlaceholder {}
        RemovePlaceholder {}
        RootTrailingPlaceholder {}
        SvgHydratedListener {}
        EmptyRootHydration {}
        StreamedEmptyHydration {}
        TextareaHydration {}
        TextareaLeadingNewline {}
        PreLeadingNewline {}
    }
}

#[component]
fn StreamedEmptyHydration() -> Element {
    rsx! {
        div {
            id: "streamed-empty-hydration",
            SuspenseBoundary {
                fallback: |_| rsx! {},
                StreamedEmptyChild {}
            }
        }
    }
}

#[component]
fn StreamedEmptyChild() -> Element {
    let _ = use_server_future(resolve_empty_stream)?().unwrap();
    #[cfg_attr(not(feature = "web"), allow(unused_mut))]
    let mut value = use_signal(String::new);

    #[cfg(feature = "web")]
    use_effect(move || {
        value.set("streamed empty hydrated".to_string());
    });

    rsx! {
        "{value}"
    }
}

async fn resolve_empty_stream() -> usize {
    #[cfg(feature = "server")]
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    0
}

// `textarea`, `pre`, and `listing` are raw-text elements: the HTML parser strips
// one newline immediately after the start tag. When the first text contribution
// starts with `\n`, the markerless walk - which reconstructs text-node positions
// by UTF-16 length - must keep the dynamic body bound to the correct text node
// so later updates land on it, rather than splitting against a length the parser
// already shortened (which mis-binds the body or raises a hydration mismatch).
// Regression for the leading-newline case of
// https://github.com/DioxusLabs/dioxus/issues/5548.
#[component]
fn TextareaLeadingNewline() -> Element {
    let mut value = use_signal(|| "BODY".to_string());

    rsx! {
        textarea { id: "textarea-leading-newline",
            "\n"
            "{value}"
        }
        button {
            id: "textarea-leading-newline-update",
            onclick: move |_| value.set("NEW".to_string()),
            "update textarea newline"
        }
    }
}

#[component]
fn PreLeadingNewline() -> Element {
    let mut value = use_signal(|| "CODE".to_string());

    rsx! {
        pre { id: "pre-leading-newline",
            "\n"
            "{value}"
        }
        button {
            id: "pre-leading-newline-update",
            onclick: move |_| value.set("CHANGED".to_string()),
            "update pre newline"
        }
    }
}

// `textarea` interprets its children as raw text, so it cannot carry hydration
// comment markers. Markerless hydration must bind the SSR-rendered text content
// and let later updates patch it. Regression for
// https://github.com/DioxusLabs/dioxus/issues/5548.
#[component]
fn TextareaHydration() -> Element {
    let mut value = use_signal(|| "initial textarea body".to_string());

    rsx! {
        textarea { id: "textarea-hydration", "{value}" }
        button {
            id: "textarea-hydration-update",
            onclick: move |_| value.set("updated textarea body".to_string()),
            "update textarea"
        }
    }
}

// A component with no SSR-visible roots. The hydration walker still needs to
// attach virtual root anchors so a client-only task can later materialize root
// text and elements without marker comments.
#[component]
fn EmptyRootHydration() -> Element {
    let mut text = use_signal(String::new);
    let mut show = use_signal(|| false);

    use_future(move || async move {
        text.set("root text ready".to_string());
        show.set(true);
    });

    rsx! {
        "{text}"
        if show() {
            div { id: "late-empty-root", "late root ready" }
        }
    }
}

// `dangerous_inner_html` renders children that are not represented in the VDOM.
// Markerless hydration must map the host element without trying to hydrate the
// raw children, while later attribute updates still replace `innerHTML`.
#[component]
fn DangerousInnerHtml() -> Element {
    let mut html =
        use_signal(|| r#"<p id="raw-inner-child">raw <strong>HTML</strong></p>"#.to_string());

    rsx! {
        div {
            id: "raw-inner-host",
            dangerous_inner_html: "{html}"
        }
        button {
            id: "swap-raw-inner",
            onclick: move |_| {
                html.set(
                    r#"<section id="raw-inner-child-updated">changed <em>HTML</em></section>"#
                        .to_string(),
                );
            },
            "swap raw"
        }
    }
}

// Adjacent dynamic texts merge into one DOM text node. The walker addresses
// each contribution by UTF-16 length via `SplitText`. After hydration we
// flip one to empty and one to non-empty; the visible text must update
// without a hydration mismatch.
#[component]
fn AdjacentDynamicTexts() -> Element {
    let mut a = use_signal(|| "AAA".to_string());
    let mut b = use_signal(|| "BBB".to_string());
    rsx! {
        div {
            id: "adjacent-texts",
            "{a}"
            "{b}"
        }
        button {
            id: "swap-adjacent",
            onclick: move |_| {
                a.set("".to_string());
                b.set("CCC".to_string());
            },
            "swap"
        }
    }
}

// Dynamic text with non-BMP characters (surrogate pairs in UTF-16). The
// walker computes `SplitText` offsets via `s.encode_utf16().count()` and the
// JS interpreter calls `Text.splitText(offset)` which is also UTF-16-indexed.
// A byte-vs-codepoint mix-up - or splitting in the middle of a surrogate
// pair - would either crash, produce unpaired surrogates, or shift text by
// one code unit per emoji. We sandwich emoji-bearing dynamics between static
// text so the merged DOM text node has known split points.
#[component]
fn Utf16Text() -> Element {
    let mut a = use_signal(|| "💧".to_string()); // 2 utf16 units
    let mut b = use_signal(|| "🌊🌊".to_string()); // 4 utf16 units
    rsx! {
        div {
            id: "utf16-text",
            "before "
            "{a}"
            " | "
            "{b}"
            " after"
        }
        button {
            id: "utf16-swap",
            onclick: move |_| {
                a.set("é💧é".to_string()); // é is 1 utf16 unit, 💧 is 2 → 4 total
                b.set("".to_string());
            },
            "swap"
        }
    }
}

// Empty dynamic text wedged between two non-empty dynamics. Before the swap
// the merged DOM text is "AAABBB" with the empty splitting nothing visible;
// the walker still has to map the empty's ElementId (via `SynthText` between
// two `SplitText` cursor moves) so the runtime can later set its text.
// After the swap, the empty becomes "MID" and the surrounding texts change
// too - the rendered result must read "leftMIDright".
#[component]
fn EmptyMiddleOfRun() -> Element {
    let mut a = use_signal(|| "AAA".to_string());
    let mut mid = use_signal(String::new);
    let mut b = use_signal(|| "BBB".to_string());
    rsx! {
        div {
            id: "empty-middle",
            "{a}"
            "{mid}"
            "{b}"
        }
        button {
            id: "fill-middle",
            onclick: move |_| {
                a.set("left".to_string());
                mid.set("MID".to_string());
                b.set("right".to_string());
            },
            "fill"
        }
    }
}

// A pure-text child component sandwiched between static text in the parent.
// SSR emits one merged text node "before MID after". The hydration walker
// must flatten the child component's text root into the parent's text-run so
// `emit_text_run` sees [static "before ", dynamic "MID" (in child scope),
// static " after"] and splits at the right offsets. Without cross-component
// flattening, the child's text would either be unmapped (no later updates
// would land on the right DOM node) or mapped to the whole merged text,
// which a later `set_text` would clobber along with the surrounding statics.
#[component]
fn ComponentInTextRun() -> Element {
    let mut value = use_signal(|| "MID".to_string());
    rsx! {
        div {
            id: "component-in-run",
            "before "
            ChildText { value: value() }
            " after"
        }
        button {
            id: "swap-component-text",
            onclick: move |_| value.set("UPDATED".to_string()),
            "swap"
        }
    }
}

#[component]
fn ChildText(value: String) -> Element {
    rsx! {
        "{value}"
    }
}

// SSR renders no bytes for an `Option::None` child. With markerless hydration
// the walker records a *virtual* placeholder for that ElementId - no
// comment node is inserted. Toggling to `Some(_)` issues `replace_with` on the
// placeholder ID, which must operate via `parent.insertBefore(real, after)`
// against the virtual entry. Toggling back to `None` reinstates a virtual
// placeholder via the diff-time `create_placeholder` path (also virtual).
#[component]
fn EmptyPlaceholderToContent() -> Element {
    let mut show = use_signal(|| false);
    rsx! {
        div {
            id: "placeholder-to-content",
            "before "
            if show() {
                span { id: "placeholder-content", "HELLO" }
            }
            " after"
        }
        button {
            id: "toggle-placeholder",
            onclick: move |_| show.toggle(),
            "toggle"
        }
    }
}

// Two markerless dynamic slots separated by a real child must not share an
// anchor chain. If the earlier slot points through the later slot, then
// materializing the later slot first makes the earlier slot insert after the
// static sibling instead of before it.
#[component]
fn SeparatedEmptyDynamicSlots() -> Element {
    let mut show_a = use_signal(|| false);
    let mut show_b = use_signal(|| false);
    rsx! {
        div {
            id: "separated-empty-slots",
            if show_a() {
                "A"
            }
            span { "S" }
            if show_b() {
                "B"
            }
        }
        button {
            id: "fill-separated-slot-b",
            onclick: move |_| show_b.set(true),
            "fill b"
        }
        button {
            id: "fill-separated-slot-a",
            onclick: move |_| show_a.set(true),
            "fill a"
        }
    }
}

// A fragment ending with an `Option::None` whose sibling later gains content
// "after" the placeholder. The placeholder's virtual `after` pointer feeds
// `insert_after`, which must position the new node at the right physical
// location *and* update the placeholder's `after` so subsequent inserts
// still land between the placeholder and what follows.
#[component]
fn PlaceholderAsTrailingSibling() -> Element {
    let mut items = use_signal(Vec::<String>::new);
    let mut count = use_signal(|| 0);
    rsx! {
        div {
            id: "trailing-placeholder",
            "HEAD"
            if count() > 0 {
                span { id: "trailing-presence", "({count})" }
            }
            for item in items.iter() {
                span { class: "trailing-item", "{item}" }
            }
        }
        button {
            id: "append-trailing",
            onclick: move |_| {
                count += 1;
                let i = count();
                items.write().push(format!("[{i}]"));
            },
            "append"
        }
    }
}

// A presence toggle that starts visible (so SSR emits real content), then
// removes itself on the client. The mutation issues `remove(id)` on the
// just-mapped element; the *replacement* placeholder gets diff-allocated
// via `create_placeholder` and must be virtual (no comment).
#[component]
fn RemovePlaceholder() -> Element {
    let mut show = use_signal(|| true);
    rsx! {
        div {
            id: "remove-placeholder",
            "edges "
            if show() {
                span { id: "removable", "PRESENT" }
            }
            " edges"
        }
        button {
            id: "hide-removable",
            onclick: move |_| show.set(false),
            "hide"
        }
    }
}

// A root-level placeholder after the final real root advances the hydration
// cursor to `null`. The virtual placeholder still needs the mount element as
// its parent so `replace_with` can append the later root.
#[component]
fn RootTrailingPlaceholder() -> Element {
    let mut show = use_signal(|| false);
    rsx! {
        button {
            id: "show-trailing-root",
            onclick: move |_| show.set(true),
            "show trailing root"
        }
        div { id: "trailing-root-before", "before trailing root" }
        if show() {
            div { id: "trailing-root-late", "late trailing root" }
        }
    }
}

// A trailing empty conditional whose append target is a *nested, non-root*
// static element. Unlike `PlaceholderAsTrailingSibling`/`SeparatedEmptyDynamicSlots`
// (where the parent is a template root), this parent is buried inside the root
// and carries no dynamic attributes, so the markerless walk maps it positionally
// with no node binding (`map_element` id == 0). Its create-time append anchor is
// a freshly allocated `ElementId` (`assign_template_anchor_ids` non-root branch),
// which hydration must bind too - otherwise materializing the conditional appends
// into an unbound node and the interpreter dereferences `undefined`
// ("Cannot read properties of undefined (reading 'insertBefore')").
//
// Regression for the docsite search-modal crash, where `if SHOW_SEARCH() { input }`
// sits several divs deep under a plain static `div` and only renders once opened.
#[component]
fn NestedTrailingPlaceholder() -> Element {
    let mut show = use_signal(|| false);
    rsx! {
        div {
            div {
                id: "nested-trailing",
                span { "HEAD" }
                if show() {
                    span { id: "nested-trailing-late", "LATE" }
                }
            }
        }
        button {
            id: "show-nested-trailing",
            onclick: move |_| show.set(true),
            "show nested trailing"
        }
    }
}

// SVG nodes implement `Element`/`EventTarget`, not `HTMLElement`. Hydrated
// listeners must bind to them.
#[component]
fn SvgHydratedListener() -> Element {
    let mut clicks = use_signal(|| 0);
    rsx! {
        svg {
            id: "hydrated-svg",
            width: "80",
            height: "40",
            view_box: "0 0 80 40",
            circle {
                id: "hydrated-circle",
                cx: "20",
                cy: "20",
                r: "14",
                onclick: move |_| clicks += 1,
            }
        }
        div { id: "svg-click-count", "svg clicks: {clicks}" }
    }
}

// Long runs of empty dynamic texts in each of the three position classes the
// walker treats differently:
//   * trailing - synth-after the last non-empty (`SynthTextAfter`, must
//     advance cursor between inserts so consecutive synths chain correctly
//     instead of reversing)
//   * leading  - synth-before the merged text node (`SynthText` insertBefore
//     against a fixed cursor, which appends in order naturally)
//   * all-empty - no merged text exists, so all synths append to the parent
//     frame via the `cursor==null` branch
// 10-wide makes a buggy walker fail loudly: any off-by-one in the chain
// reverses the visible order once the empties become non-empty.
#[component]
fn LongEmptyRuns() -> Element {
    let mut a = use_signal(String::new);
    let mut b = use_signal(String::new);
    let mut c = use_signal(String::new);
    let mut d = use_signal(String::new);
    let mut e = use_signal(String::new);
    let mut f = use_signal(String::new);
    let mut g = use_signal(String::new);
    let mut h = use_signal(String::new);
    let mut i = use_signal(String::new);
    let mut j = use_signal(String::new);
    rsx! {
        div {
            id: "trailing-empties-10",
            "HEAD"
            "{a}" "{b}" "{c}" "{d}" "{e}" "{f}" "{g}" "{h}" "{i}" "{j}"
        }
        div {
            id: "leading-empties-10",
            "{a}" "{b}" "{c}" "{d}" "{e}" "{f}" "{g}" "{h}" "{i}" "{j}"
            "TAIL"
        }
        div {
            id: "all-empties-10",
            "{a}" "{b}" "{c}" "{d}" "{e}" "{f}" "{g}" "{h}" "{i}" "{j}"
        }
        button {
            id: "fill-runs",
            onclick: move |_| {
                a.set("[a]".to_string());
                b.set("[b]".to_string());
                c.set("[c]".to_string());
                d.set("[d]".to_string());
                e.set("[e]".to_string());
                f.set("[f]".to_string());
                g.set("[g]".to_string());
                h.set("[h]".to_string());
                i.set("[i]".to_string());
                j.set("[j]".to_string());
            },
            "fill"
        }
    }
}
