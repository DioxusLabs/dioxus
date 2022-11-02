use dioxus_core::prelude::*;

fn app(cx: Scope) -> Element {
    todo!();
    // render! {
    //      Suspend {
    //          delay: Duration::from_millis(100),
    //          fallback: rsx! { "Loading..." },
    //          ChildAsync {}
    //          ChildAsync {}
    //          ChildAsync {}
    //      }
    // }
}

async fn ChildAsync(cx: Scope<'_>) -> Element {
    todo!()
}

#[test]
fn it_works() {
    let mut dom = VirtualDom::new(app);

    let mut mutations = vec![];
    dom.rebuild(&mut mutations);
}
