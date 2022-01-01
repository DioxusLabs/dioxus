use dioxus::prelude::*;

fn main() {
    let mut dom = VirtualDom::new(app);
    dom.rebuild();

    rink::render_vdom(&mut dom).unwrap();
}

fn app(cx: Scope) -> Element {
    let count = use_state(&cx, || 0);

    use_future(&cx, || {
        let set_count = count.setter();
        let mut mycount = 0;
        let update = cx.schedule_update();
        async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                mycount += 1;
                set_count(mycount);
                update();
            }
        }
    });

    cx.render(rsx! {
        div { width: "100%",
            div { width: "50%", height: "5px", background_color: "blue", justify_content: "center", align_items: "center",
                "Hello {count}!"
            }
            div { width: "50%", height: "10px", background_color: "red", justify_content: "center", align_items: "center",
                "Hello {count}!"
            }
        }
    })
}

// use_future(&cx, || {
//         let set_count = count.setter();
//         let mut mycount = 0;
//         let update = cx.schedule_update();
//         async move {
//             loop {
//                 tokio::time::sleep(std::time::Duration::from_millis(100)).await;
//                 mycount += 1;
//                 set_count(mycount);
//                 update();
//             }
//         }
//     });
