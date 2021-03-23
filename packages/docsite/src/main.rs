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
    let body = match props.route {
        Routes::Homepage => ctx.render(rsx!(
            div {
                Homepage {}
            }
        )),
        Routes::ExampleList => ctx.render(rsx!(
        div {
            ExampleList {}
            }
        )),
    };

    ctx.render(rsx!(
        div {
            Header {}
            {body}
            {
            }
            Footer {}
        }
    ))
};

#[derive(Debug, PartialEq, Props)]
struct HeaderProp {
    selected: Routes,
}

static Header: FC<()> = |ctx, _| {
    ctx.render(rsx! {
        div {

        }
    })
};

static Footer: FC<()> = |ctx, _| {
    ctx.render(rsx! {
        div {

        }
    })
};

static Homepage: FC<()> = |ctx, _| {
    ctx.render(rsx! {
        div {

        }
    })
};

static ExampleList: FC<()> = |ctx, _| {
    ctx.render(rsx! {
        div {

        }
    })
};
