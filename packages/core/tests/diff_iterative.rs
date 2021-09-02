//! tests to prove that the iterative implementation works

use dioxus::prelude::*;

mod test_logging;
use dioxus_core as dioxus;
use dioxus_html as dioxus_elements;

const LOGGING_ENABLED: bool = false;

#[async_std::test]
async fn test_iterative_create_components() {
    static App: FC<()> = |cx| {
        // test root fragments
        cx.render(rsx! {
            Child { "abc1" }
            Child { "abc2" }
            Child { "abc3" }
        })
    };

    fn Child(cx: Context<()>) -> DomTree {
        // test root fragments, anchors, and ChildNode type
        cx.render(rsx! {
            h1 {}
            div { {cx.children()} }
            Fragment {
                Fragment {
                    Fragment {
                        "wozza"
                    }
                }
            }
            {(0..0).map(|_f| rsx!{ div { "walalla"}})}
            p {}
        })
    }

    test_logging::set_up_logging(LOGGING_ENABLED);

    let mut dom = VirtualDom::new(App);

    let mutations = dom.rebuild_async().await;
    dbg!(mutations);

    let mutations = dom.diff();
    dbg!(mutations);
}
