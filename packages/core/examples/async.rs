use dioxus_core::prelude::*;

fn main() {
    let mut dom = VirtualDom::new(App);
    dom.rebuild();
}

const App: FC<()> = |cx, props| {
    let id = cx.scope_id();
    cx.submit_task(Box::pin(async move { id }));

    let (handle, contents) = use_task(cx, || async { "hello world".to_string() });

    todo!()
};
