//! This example shows how to implement a simple drag-and-drop kanban board using Dioxus.
//! You can drag items between different categories and edit their contents.
//!
//! This example uses the `.data_transfer()` API to handle drag-and-drop events. When an item is dragged,
//! its ID is stored in the data transfer object. When the item is dropped into a new category, its ID is retrieved
//! from the data transfer object and used to update the item's category.
//!
//! Note that in a real-world application, you'll want more sophisticated drop handling, such as visual
//! feedback during dragging, and better drop-zone detection to allow dropping *between* items.

use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

struct Item {
    id: usize,
    name: String,
    category: String,
    contents: String,
}

fn app() -> Element {
    let mut items = use_signal(initial_kanban_data);

    rsx! {
        div {
            display: "flex",
            gap: "20px",
            flex_direction: "row",
            for category in ["A", "B", "C"] {
                div {
                    class: "category",
                    display: "flex",
                    flex_direction: "column",
                    gap: "10px",
                    padding: "10px",
                    flex_grow: "1",
                    border: "2px solid black",
                    min_height: "300px",
                    background_color: "#f0f0f0",
                    ondragover: |e| e.prevent_default(),
                    ondrop: move |e| {
                        if let Some(item_id) = e.data_transfer().get_data("text/plain").and_then(|data| data.parse::<usize>().ok()) {
                            if let Some(pos) = items.iter().position(|item| item.id == item_id) {
                                items.write()[pos].category = category.to_string();
                            }
                        }
                    },
                    h2 { "Category: {category}" }
                    for (index, item) in items.iter().enumerate().filter(|item| item.1.category == category) {
                        div {
                            key: "{item.id}",
                            width: "200px",
                            height: "50px",
                            border: "1px solid black",
                            padding: "10px",
                            class: "item",
                            draggable: "true",
                            background: "white",
                            cursor: "grab",
                            ondragstart: move |e| {
                                let id = items.read()[index].id.to_string();
                                e.data_transfer().set_data("text/plain", &id).unwrap();
                            },
                            pre { webkit_user_select: "none", "{item.name}" }
                            input {
                                r#type: "text",
                                value: "{item.contents}",
                                oninput: move |e| {
                                    items.write()[index].contents = e.value();
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn initial_kanban_data() -> Vec<Item> {
    vec![
        Item {
            id: 1,
            name: "Item 1".into(),
            category: "A".into(),
            contents: "This is item 1".into(),
        },
        Item {
            id: 2,
            name: "Item 2".into(),
            category: "A".into(),
            contents: "This is item 2".into(),
        },
        Item {
            id: 3,
            name: "Item 3".into(),
            category: "A".into(),
            contents: "This is item 3".into(),
        },
        Item {
            id: 4,
            name: "Item 4".into(),
            category: "B".into(),
            contents: "This is item 4".into(),
        },
        Item {
            id: 5,
            name: "Item 5".into(),
            category: "B".into(),
            contents: "This is item 5".into(),
        },
        Item {
            id: 6,
            name: "Item 6".into(),
            category: "C".into(),
            contents: "This is item 6".into(),
        },
    ]
}
