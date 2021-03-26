use dioxus_ssr::{
    prelude::{builder::IntoDomTree, dioxus::events::on::MouseEvent, *},
    TextRenderer,
};
mod components {
    mod app;
}

fn main() {
    TextRenderer::new(App);
}

#[derive(Debug, PartialEq)]
enum Route {
    Homepage,
    Examples,
}

#[derive(Debug, PartialEq, Props)]
struct AppProps {
    route: Route,
}

static App: FC<AppProps> = |ctx, props| {
    let body = match props.route {
        Route::Homepage => ctx.render(rsx! {
            div {
                "Some Content"
            }
        }),

        Route::Examples => ctx.render(rsx! {
            div {
                "Other Content"
            }
        }),
    };

    ctx.render(rsx!(
        div {
            Header {}
            {body}
            ul {
                {(0..10).map(|f| rsx!{
                    li {
                        "this is list item {f}"
                    }
                })}
            }
        }
    ))
};

static Header: FC<()> = |ctx, _| {
    ctx.render(rsx! {
        div {

        }
    })
};

mod example {
    use super::*;
    static ExampleUsage: FC<()> = |ctx, props| {
        // direct rsx!
        let body = rsx! {
            div {}
        };

        // rendered rsx!
        let top = ctx.render(rsx! {
            div {
                "ack!"
            }
        });

        // listy rsx
        let list2 = (0..10).map(|f| {
            rsx! {
                div {}
            }
        });

        // rendered list rsx
        let list = (0..10).map(|f| {
            ctx.render(rsx! {
                div {}
            })
        });

        ctx.render(rsx!(
            div {
                Header {}
                {body}
                {top}
                {list}
                {list2}
                // inline rsx
                {rsx!{
                    div { "hello" }
                }}
            }
        ))
    };
}
