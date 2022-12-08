use dioxus::core::RenderReturn;
use dioxus::prelude::*;
use dioxus_tui::query::Query;
use dioxus_tui::Size;

fn main() {
    dioxus_tui::launch(app);
}

fn app(cx: Scope) -> Element {
    let hue = use_state(cx, || 0.0);
    let brightness = use_state(cx, || 0.0);
    let tui_query: &Query = cx.consume_context().unwrap();
    // disable templates so that every node has an id and can be queried
    cx.render(rsx! {
        div{
            width: "100%",
            background_color: "hsl({hue}, 70%, {brightness}%)",
            onmousemove: move |evt| {
                if let RenderReturn::Sync(Ok(node))=cx.root_node(){
                    let node = tui_query.get(node.root_ids[0].get());
                    let Size{width, height} = node.size().unwrap();
                    let pos = evt.inner().element_coordinates();
                    hue.set((pos.x as f32/width as f32)*255.0);
                    brightness.set((pos.y as f32/height as f32)*100.0);
                }
            },
            "hsl({hue}, 70%, {brightness}%)",
        }
    })
}
