//! This example shows how you can chain memos together to create a tree of memoized values.
//!
//! Memos will also pause when their parent component pauses, so if you have a memo that depends on a signal, and the
//! signal pauses, the memo will pause too.

use dioxus::prelude::*;

fn main() {
    launch(app);
}

fn app() -> Element {
    let mut value = use_signal(|| 0);
    let mut depth = use_signal(|| 0_usize);
    let items = use_memo(move || (0..depth()).map(|f| f as _).collect::<Vec<isize>>());
    let state = use_memo(move || value() + 1);

    println!("rendering app");

    rsx! {
        button { onclick: move |_| value += 1, "Increment" }
        button { onclick: move |_| depth += 1, "Add depth" }
        button { onclick: move |_| depth -= 1, "Remove depth" }
        if depth() > 0 {
            Child { depth, items, state }
        }
    }
}

#[component]
fn Child(state: Memo<isize>, items: Memo<Vec<isize>>, depth: ReadOnlySignal<usize>) -> Element {
    // These memos don't get re-computed when early returns happen
    let state = use_memo(move || state() + 1);
    let item = use_memo(move || items()[depth() - 1]);
    let depth = use_memo(move || depth() - 1);

    println!("rendering child: {}", depth());

    rsx! {
        h3 { "Depth({depth})-Item({item}): {state}"}
        if depth() > 0 {
            Child { depth, state, items }
        }
    }
}
