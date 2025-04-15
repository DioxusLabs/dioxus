use dioxus::prelude::*;

pub fn launch() {
    dioxus::launch(app);
}

static CSS1: Asset = asset!("/assets/test.css");
static CSS2: Asset = asset!("/assets/test.css");
static CSS3: Asset = asset!("/assets/test.css");

fn app() -> Element {
    let mut items: Signal<Vec<String>> = use_signal(|| vec![]);

    rsx! {
        div {
            link { href: CSS1, rel: "stylesheet" }
            link { href: CSS2, rel: "stylesheet" }
            link { href: CSS3, rel: "stylesheet" }
            h1 { "Build a todo list!" }
            h1 { "Build a todo list!" }
            h1 { "Build a todo list!" }
            h1 { "Build a todo list!" }
            button {
                onclick: move |_| async move {
                    // let res = request_item(123).await.unwrap();
                    let res = "Item".to_string();
                    items.write().push(res);
                },
                "Add Item"
            }


            button {
                onclick: move |_| {
                    items.write().clear();
                },
                "Clear Items"
            }


            for item in items.iter() {
                li { "{item}" }
            }
        }
    }
}

// #[server(endpoint = "request_item")]
// pub async fn request_item(val: i32) -> Result<String, ServerFnError> {
//     dioxus::subsecond::call(|| {
//         Ok("hotpatchy frontend frontend frontend frontend frontend!!!".to_string())
//     })
// }

// fn app() -> Element {
//     let mut count = use_signal(|| 0);

//     rsx! {
//         div { style: "display: flex; flex-direction: column; align-items: center; justify-content: center;",
//             h1 { "Apple: {count} ???" }
//             button { onclick: move |_| count += 1, "Incr" }
//             button { onclick: move |_| count -= 1, "Decr" }
//             img {
//                 width: "300px",
//                 src: "https://rustacean.net/assets/rustacean-flat-happy.png",
//             }
//         }
//         div { style: "display: flex; flex-direction: column; align-items: center; justify-content: center;",
//             div { style: "background-color: red",
//                 for x in 0..1 {
//                     Child { id: x + 1, opt: "List entry", color: "gri" }
//                 }
//             }
//             div { style: "background-color: orange",
//                 for x in 0..1 {
//                     Child { id: x + 1, opt: "List entry", color: "blue" }
//                 }
//             }
//             div { style: "background-color: yellow",
//                 for x in 0..1 {
//                     Child { id: x + 1, opt: "List entry", color: "yellow" }
//                 }
//             }
//             div { style: "background-color: green",
//                 for x in 0..1 {
//                     Child { id: x + 10, opt: "List entry", color: "orange" }
//                 }
//             }
//             div { style: "background-color: blue",
//                 for x in 0..1 {
//                     Child { id: x + 10, opt: "List entry", color: "bluebleu" }
//                 }
//             }
//             div { style: "background-color: indigo",
//                 for x in 0..1 {
//                     Child { id: x + 10, opt: "List entry", color: "magentaaa" }
//                 }
//             }
//         }
//     }
// }

// #[component]
// fn Child(id: u32, opt: String, color: String) -> Element {
//     let mut count = use_signal(|| 0);

//     rsx! {
//         div {
//             h3 { "Chil!!!!!!!!!! {id} - {opt} - {color} - {color} - {color}" }
//             p { "count: {count}" }
//             button {
//                 onclick: move |_| {
//                     count += id;
//                 },
//                 "Increment Count"
//             }
//         }
//     }
// }
// #[component]
// fn Child2(id: u32, opt: String) -> Element {
//     rsx! {
//         div { "oh lordy!" }
//         div { "Hello ?? child2s: {id} - {opt} ?" }
//     }
// }

// #[component]
// fn Child3(id: u32, opt: String) -> Element {
//     rsx! {
//         div { "Hello ?? child: {id} - {opt} ?" }
//     }
// }

// #[component]
// fn Child4(id: u32, opt: String) -> Element {
//     rsx! {
//         div { "Hello ?? child: {id} - {opt} ?" }
//         div { "Hello ?? child: {id} - {opt} ?" }
//         div { "Hello ?? child: {id} - {opt} ?" }
//     }
// }

// #[component]
// fn ZoomComponent() -> Element {
//     // use dioxus::desktop::window;
//     // button { onclick: move |_| window().set_zoom_level(1.0), "Zoom 1x" }
//     // button { onclick: move |_| window().set_zoom_level(1.5), "Zoom 1.5x" }
//     // button { onclick: move |_| window().set_zoom_level(2.0), "Zoom 2x" }
//     // button { onclick: move |_| window().set_zoom_level(3.0), "Zoom 3x" }
//     rsx! {
//         div { "Zoom me!" }
//     }
// }

// fn app() -> Element {
//     let mut items: Signal<Vec<String>> = use_signal(|| vec![]);

//     rsx! {
//         div {
//             h1 { "Build a todo list!" }
//             h1 { "Build a todo list!" }
//             h1 { "Build a todo list!" }
//             button {
//                 onclick: move |_| async move {
//                     // let res = request_item(123).await.unwrap();
//                     items.write().push("Item".to_string());
//                 },
//                 "Add Item"
//             }

//             for item in items.iter() {
//                 li { "{item}" }
//             }
//         }
//     }
// }
