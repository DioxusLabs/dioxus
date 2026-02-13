use dioxus::prelude::*;
use dioxus_core::generation;

#[derive(Debug, Clone, dioxus_stores::Store)]
struct ListItem {
    id: usize,
    text: String,
}

/// Removing a non-last item from a Vec store should not panic during memoization.
#[test]
fn vec_store_remove_non_last_item() {
    fn app() -> Element {
        let mut store = use_store(|| {
            vec![
                ListItem {
                    id: 0,
                    text: "Item 0".to_string(),
                },
                ListItem {
                    id: 1,
                    text: "Item 1".to_string(),
                },
            ]
        });

        // On the second render, remove the first item.
        if generation() > 0 {
            store.remove(0);
        }

        rsx! {
            for item in store.iter() {
                li { key: "{item.id()}",
                    ListItemElement { list_item: item }
                }
            }
        }
    }

    #[component]
    fn ListItemElement(list_item: ReadSignal<ListItem>) -> Element {
        rsx! {
            div { "{list_item.read().text}" }
        }
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild(&mut dioxus_core::NoOpMutations);
    // Second render triggers remove(0), which previously panicked during memoization
    dom.mark_dirty(ScopeId::APP);
    _ = dom.render_immediate_to_vec();
}
