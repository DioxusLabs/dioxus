use dioxus::prelude::*;

fn main() {
    let mut vdom = VirtualDom::new(example);
    vdom.rebuild();

    let out = dioxus::ssr::render_vdom_cfg(&vdom, |c| c.newline(true).indent(true));
    println!("{}", out);
}

fn example(cx: Scope) -> Element {
    let items = use_state(&cx, || {
        vec![Thing {
            a: "asd".to_string(),
            b: 10,
        }]
    });

    let things = use_ref(&cx, || {
        vec![Thing {
            a: "asd".to_string(),
            b: 10,
        }]
    });
    let things_list = things.read();

    let mything = use_ref(&cx, || Some(String::from("asd")));
    let mything_read = mything.read();

    cx.render(rsx!(
        div {
            div {
                id: "asd",
                "your neighborhood spiderman"

                items.iter().cycle().take(5).map(|f| rsx!{
                    div {
                        "{f.a}"
                    }
                })

                things_list.iter().map(|f| rsx!{
                    div {
                        "{f.a}"
                    }
                })

                mything_read.as_ref().map(|f| rsx!{
                    div {
                       "{f}"
                    }
                })
            }
        }
    ))
}

struct Thing {
    a: String,
    b: u32,
}
