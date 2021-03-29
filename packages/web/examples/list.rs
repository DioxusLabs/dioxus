use std::collections::{BTreeMap, BTreeSet, HashMap};

use dioxus::{events::on::MouseEvent, prelude::*};
use dioxus_core as dioxus;
use dioxus_web::WebsysRenderer;

fn main() {
    wasm_logger::init(wasm_logger::Config::new(log::Level::Debug));
    console_error_panic_hook::set_once();
    wasm_bindgen_futures::spawn_local(async move {
        WebsysRenderer::new_with_props(App, ())
            .run()
            .await
            .expect("major crash");
    });
}

use lazy_static::lazy_static;
lazy_static! {
    static ref DummyData: BTreeMap<usize, String> = {
        let vals = vec![
            "abc123", //
            "abc124", //
            "abc125", //
            "abc126", //
            "abc127", //
            "abc128", //
            "abc129", //
            "abc1210", //
            "abc1211", //
            "abc1212", //
            "abc1213", //
            "abc1214", //
            "abc1215", //
            "abc1216", //
            "abc1217", //
            "abc1218", //
            "abc1219", //
            "abc1220", //
            "abc1221", //
            "abc1222", //
        ];
        vals.into_iter()
            .map(ToString::to_string)
            .enumerate()
            .collect()
    };
}

static App: FC<()> = |ctx, _| {
    let items = use_state_new(&ctx, || DummyData.clone());

    // handle new elements
    let add_new = move |_| {
        items.modify(|m| {
            let k = m.len();
            let v = match (k % 3, k % 5) {
                (0, 0) => "FizzBuzz".to_string(),
                (0, _) => "Fizz".to_string(),
                (_, 0) => "Buzz".to_string(),
                _ => k.to_string(),
            };
            m.insert(k, v);
        })
    };

    let elements = items.iter().map(|(k, v)| {
        rsx! {
            ListHelper {
                name: k,
                value: v
                onclick: move |_| {
                    let key = k.clone();
                    items.modify(move |m| { m.remove(&key); } )
                }
            }
        }
    });

    ctx.render(rsx!(
        div {
            h1 {"Some list"}
            button {
                "Remove all"
                onclick: move |_| items.set(BTreeMap::new())
            }
            button {
                "add new"
                onclick: {add_new}
            }
            ul {
                {elements}
            }
        }
    ))
};

#[derive(Props)]
struct ListProps<'a, F: Fn(MouseEvent) + 'a> {
    name: &'a usize,
    value: &'a str,
    onclick: F,
}

impl<F: Fn(MouseEvent)> PartialEq for ListProps<'_, F> {
    fn eq(&self, other: &Self) -> bool {
        // no references are ever the same
        false
    }
}

fn ListHelper<F: Fn(MouseEvent)>(ctx: Context, props: &ListProps<F>) -> DomTree {
    let k = props.name;
    let v = props.value;
    ctx.render(rsx! {
        li {
            class: "flex items-center text-xl"
            key: "{k}"
            span { "{k}: {v}" }
            button {
                "__ Remove"
                onclick: {&props.onclick}
            }
        }
    })
}
