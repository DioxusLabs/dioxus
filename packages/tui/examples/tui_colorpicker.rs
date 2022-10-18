use dioxus::core_macro::rsx_without_templates;
use dioxus::prelude::*;
use dioxus_tui::query::Query;
use dioxus_tui::Size;

fn main() {
    dioxus_tui::launch(app);
}

fn app(cx: Scope) -> Element {
    let hue = use_state(&cx, || 0.0);
    let brightness = use_state(&cx, || 0.0);
    let tui_query: Query = cx.consume_context().unwrap();
    // disable templates so that every node has an id and can be queried
    cx.render(rsx_without_templates! {
        div{
            width: "100%",
            background_color: "hsl({hue}, 70%, {brightness}%)",
            onmousemove: move |evt| {
                let node = tui_query.get(cx.root_node().mounted_id());
                let Size{width, height} = node.size().unwrap();
                let pos = evt.data.element_coordinates();
                hue.set((pos.x as f32/width as f32)*255.0);
                brightness.set((pos.y as f32/height as f32)*100.0);
            },
            "hsl({hue}, 70%, {brightness}%)",
        }
    })
}
