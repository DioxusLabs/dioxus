use dioxus::prelude::*;
use dioxus::ssr;

fn main() {
    let mut vdom = VirtualDom::new(App);
    vdom.rebuild_in_place();
    println!("{}", ssr::render_root(&vdom));
}

const App: FC<()> = |cx| {
    cx.render(rsx!(
        div {
            h1 { "Title" }
            p { "Body" }
        }
    ))
};
