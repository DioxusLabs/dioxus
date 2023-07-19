use dioxus::core::RenderReturn;
use dioxus::prelude::*;
use dioxus_tui::DioxusElementToNodeId;
use dioxus_tui::Query;
use dioxus_tui::Size;

fn main() {
    dioxus_tui::launch(app);
}

fn app(cx: Scope) -> Element {
    let hue = use_state(cx, || 0.0);
    let brightness = use_state(cx, || 0.0);
    let tui_query: Query = cx.consume_context().unwrap();
    let mapping: DioxusElementToNodeId = cx.consume_context().unwrap();
    // disable templates so that every node has an id and can be queried
    cx.render(rsx! {
        div{
            width: "100%",
            background_color: "hsl({hue}, 70%, {brightness}%)",
            onmousemove: move |evt| {
                if let RenderReturn::Ready(node) = cx.root_node() {
                    if let Some(id) = node.root_ids.borrow().get(0).cloned() {
                        let node = tui_query.get(mapping.get_node_id(id).unwrap());
                        let Size{width, height} = node.size().unwrap();
                        let pos = evt.inner().element_coordinates();
                        hue.set((pos.x as f32/width as f32)*255.0);
                        brightness.set((pos.y as f32/height as f32)*100.0);
                    }
                }
            },
            "hsl({hue}, 70%, {brightness}%)",
        }
    })
}
