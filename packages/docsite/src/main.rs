use dioxus_ssr::{prelude::*, TextRenderer};

fn main() {
    TextRenderer::new(App);
}

#[derive(Debug, PartialEq)]
enum Routes {
    Homepage,
    ExampleList,
}

#[derive(Debug, PartialEq, Props)]
struct AppProps {
    route: Routes,
}

trait Blah {}
impl<'a, G> Blah for LazyNodes<'a, G> where G: for<'b> FnOnce(&'b NodeCtx<'a>) -> VNode<'a> + 'a {}

static App: FC<AppProps> = |ctx, props| {
    //
    let body = rsx! {
        div {}
    };

    let top = rsx! {
        div {}
    };

    ctx.render(rsx!(
        div {
            Header {}
            {body}
            {top}
            {rsx!{
                div {
                    "you ugl"
                }
            }}
        }
    ))
};

static Header: FC<()> = |ctx, _| {
    ctx.render(rsx! {
        div {

        }
    })
};
