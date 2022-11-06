use std::time::Duration;

use dioxus_core::*;

#[tokio::test]
async fn it_works() {
    let mut dom = VirtualDom::new(app);

    let mut mutations = vec![];
    dom.rebuild(&mut mutations);

    println!("mutations: {:?}", mutations);

    dom.wait_for_work().await;
}

fn app(cx: Scope) -> Element {
    cx.spawn(async {
        for x in 0..10 {
            tokio::time::sleep(Duration::from_secs(1)).await;
            println!("Hello, world! {x}");
        }
    });

    cx.spawn(async {
        for x in 0..10 {
            tokio::time::sleep(Duration::from_millis(500)).await;
            println!("Hello, world does! {x}");
        }
    });

    None
}
