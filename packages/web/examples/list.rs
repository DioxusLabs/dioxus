use std::collections::HashMap;

use dioxus::prelude::*;
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
    static ref DummyData: HashMap<String, String> = {
        let vals = vec![
            ("0 ", "abc123"),
            ("1 ", "abc124"),
            ("2 ", "abc125"),
            ("3 ", "abc126"),
            ("4 ", "abc127"),
            ("5 ", "abc128"),
            ("6 ", "abc129"),
            ("7 ", "abc1210"),
            ("8 ", "abc1211"),
            ("9 ", "abc1212"),
            ("10 ", "abc1213"),
            ("11 ", "abc1214"),
            ("12 ", "abc1215"),
            ("13 ", "abc1216"),
            ("14 ", "abc1217"),
            ("15 ", "abc1218"),
            ("16 ", "abc1219"),
            ("17 ", "abc1220"),
            ("18 ", "abc1221"),
            ("19 ", "abc1222"),
        ];
        vals.into_iter()
            .map(|(a, b)| (a.to_string(), b.to_string()))
            .collect()
    };
}

static App: FC<()> = |ctx, _| {
    let items = use_state(&ctx, || DummyData.clone());

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
            m.insert(k.to_string(), v);
        })
    };

    let elements = items.iter().map(|(k, v)| {
        rsx! {
            li {
                span {"{k}: {v}"}
                button {
                    "Remove"
                    onclick: move |_| {
                        let key_to_remove = k.clone();
                        items.modify(move |m| { m.remove(&key_to_remove); } )
                    }
                }
            }
        }
    });

    ctx.render(rsx!(
        div {
            h1 {"Some list"}

            // button  to add new item
            button {
                "add new"
                onclick: {add_new}
            }

            // list elements
            ul {
                {elements}
            }
        }
    ))
};
