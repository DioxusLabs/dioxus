use dioxus::prelude::*;
use dioxus_renderer_oracle::RendererOracle;

#[test]
fn bool_test() {
    fn app() -> Element {
        rsx! { div { hidden: false } }
    }

    let mut dom = VirtualDom::new(app);
    let mut oracle = RendererOracle::new();
    oracle.rebuild(&mut dom);
    oracle.assert_matches(app);
}
