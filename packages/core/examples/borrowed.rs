//! Demonstrate that borrowed data is possible as a property type
//! Borrowing (rather than cloning) is very important for speed and ergonomics.
//!
//! It's slightly more advanced than just cloning, but well worth the investment.
//!
//! If you use the FC macro, we handle the lifetimes automatically, making it easy to write efficient & performant components.

fn main() {}

use dioxus_core::prelude::*;

struct Props {
    items: Vec<ListItem>,
}

struct ListItem {
    name: String,
    age: u32,
}

fn app(ctx: Context, props: &Props) -> DomTree {
    let (f, setter) = use_state(&ctx, || 0);

    ctx.render(move |b| {
        let mut root = builder::div(b);
        for child in &props.items {
            // notice that the child directly borrows from our vec
            // this makes lists very fast (simply views reusing lifetimes)
            root = root.child(builder::virtual_child(
                b,
                ChildProps {
                    item: child,
                    item_handler: setter,
                },
                // <ChildItem item=child hanldler=setter />
                child_item,
            ));
        }
        root.finish()
    })
}

type StateSetter<T> = dyn Fn(T);

struct ChildProps<'a> {
    // Pass down complex structs
    item: &'a ListItem,

    // Even pass down handlers!
    item_handler: &'a StateSetter<i32>,
}

fn child_item(ctx: Context, props: &ChildProps) -> DomTree {
    todo!()
    //     ctx.render(rsx! {
    //         div {
    //             item: child,
    //             handler: setter,
    //             abc: 123,
    //             onclick: props.item_handler,

    //             h1 { "abcd123" }
    //             h2 { "abcd123" }
    //             div {
    //                 "abcd123"
    //                 h2 { }
    //                 p { }
    //             },
    //         }
    //     })
}

/*




rsx! {
    ChildItem {
        // props
        item: child, handler: setter,

        // children
        div { class:"abcd", abc: 123 },
        div { class:"abcd", abc: 123 },

        // Auto-text coercion
        "eyo matie {abc}",

        // Anything that accepts Into<VChild>
        {},
    }
}

// dreaming of this syntax
#[derive(Properties)]
struct ChildProps<'a> {
    username:  &'a str,
    item_handler: &'a dyn Fn(i32),
}

fn child_item(ctx: Context, props: &ChildProps) -> DomTree {
    ctx.render(rsx! {
        div {
            class: "abc123",
            abc: 123,
            onclick: props.item_handler,

            h1 { "Hello, {props.username}!" },
            h2 { "Welcome the RSX syntax" },
            div {
                h3 { "This is a subheader" }
                button {
                  onclick: props.handler,
                  "This is a button"
                  }
                "This is child text"
            },
        }
    })
}

// This is also nice

#[dioxus::component]
static CHILD: FC = |ctx, username: &str, handler: &dyn Fn(i32)| {
    ctx.render(rsx! {
        div {
            class: "abc123",
            abc: 123,
            onclick: handler,

            h1 { "Hello, {username}!" },
            h2 { "Welcome the RSX syntax" },
            div {
                h3 { "This is a subheader" }
                button {
                  onclick: props.handler,
                  "This is a button"
                  }
                "This is child text"
            },
        }
    })
}
Menlo, Monaco, 'Courier New', monospace



struct Item {
  name: String,
  content: String,
}

#[dioxus::live_component]
static CHILD: FC = |ctx, username: &str, handler: &dyn Fn(i32)| {
      // return lazy nodes or
      let ssr = ctx.suspend(async {
        let data = fetch("https://google.com")
                      .await?
                      .json::<Item>()
                      .await?;
          rsx! {
          div {
            h1 { "Welcome: {data.name}" }
            p { "Content: \n {data.content}" }
          }
          }
      });

    ctx.render(rsx! {
        div {
            class: "abc123",
            abc: 123,
            onclick: handler,

            h1 { "Hello, {username}!" },
            h2 { "Welcome the RSX syntax" },
            div {
                h3 { "This is a subheader" }
                button {
                  onclick: props.handler,
                  "This is a button"
                  }
                {ssr}
            },
        }
    })
}

*/
