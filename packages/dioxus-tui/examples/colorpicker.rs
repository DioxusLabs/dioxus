use dioxus::dioxus_core::RenderReturn;
use dioxus::prelude::*;
use dioxus_tui::DioxusElementToNodeId;
use dioxus_tui::Query;
use dioxus_tui::Size;

fn main() {
    dioxus_tui::launch(app);
}

fn app() -> Element {
    let mut hue = use_signal(|| 0.0);
    let mut brightness = use_signal(|| 0.0);
    let tui_query: Query = consume_context();
    let mapping: DioxusElementToNodeId = consume_context();

    // disable templates so that every node has an id and can be queried
    rsx! {
        div {
            width: "100%",
            background_color: "hsl({hue}, 70%, {brightness}%)",
            onmousemove: move |evt| {
                todo!()
                // if let RenderReturn::Ready(node) = root_node() {
                //     if let Some(id) = node.root_ids.borrow().first().cloned() {
                //         let node = tui_query.get(mapping.get_node_id(id).unwrap());
                //         let Size { width, height } = node.size().unwrap();
                //         let pos = evt.inner().element_coordinates();
                //         hue.set((pos.x as f32 / width as f32) * 255.0);
                //         brightness.set((pos.y as f32 / height as f32) * 100.0);
                //     }
                // }
            },
            "hsl({hue}, 70%, {brightness}%)"
        }
    }
}
