use dioxus::prelude::*;

pub fn launch() {
    dioxus::launch(app);
}

fn app() -> Element {
    let count = 123;

    dioxus::logger::info!("Rendering component!");

    // function pointers correlate directly to exports.table.get(pointer!)
    //
    // object.keys(patch_exports) -> gives us names of functions
    //
    // we end up making a map of original ptr to... what??
    //
    // if we load the new module into the same table then its pointers should be valid too?
    //
    // the jump table can continue to be a map of ptr -> ptr
    //
    // but we somehow need to get the exports as pointers too
    //
    // the exported functions return a "pointer object" whose `name` field *is the pointer* (but as a string)
    //
    // so in theory we....
    //  object.values(patch.exports) => [name, ptr]
    //  source.exports[name].name => ptr
    //
    // call patchJumpTable({ptr: ptr})

    dioxus::logger::info!("fn ptr of app is {:?}", app as *const fn() -> Element);
    dioxus::logger::info!("fn ptr of child is {:?}", Child as *const fn() -> Element);
    dioxus::logger::info!("fn ptr of child2 is {:?}", Child2 as *const fn() -> Element);
    dioxus::logger::info!("fn ptr of child3 is {:?}", Child3 as *const fn() -> Element);

    rsx! {
        "hi {count}"
        div {
            for x in 0..3 {
                Child { id: x + 1, opt: "List entry" }
            }
        }
    }
}

#[component]
fn Child(id: u32, opt: String) -> Element {
    let mut count = use_signal(|| 0);

    rsx! {
        div {
            h3 { "Child: {id} - {opt}" }
            p { "count: {count}" }
            button {
                onclick: move |_| {
                    count += id;
                },
                "Increment Count"
            }
        }
    }
}
#[component]
fn Child2(id: u32, opt: String) -> Element {
    rsx! {
        div { "oh lordy!" }
        div { "Hello ?? child2s: {id} - {opt} ?" }
    }
}

#[component]
fn Child3(id: u32, opt: String) -> Element {
    rsx! {
        div { "Hello ?? child: {id} - {opt} ?" }
    }
}

#[component]
fn Child4(id: u32, opt: String) -> Element {
    rsx! {
        div { "Hello ?? child: {id} - {opt} ?" }
        div { "Hello ?? child: {id} - {opt} ?" }
        div { "Hello ?? child: {id} - {opt} ?" }
    }
}

#[component]
fn ZoomComponent() -> Element {
    // use dioxus::desktop::window;
    // button { onclick: move |_| window().set_zoom_level(1.0), "Zoom 1x" }
    // button { onclick: move |_| window().set_zoom_level(1.5), "Zoom 1.5x" }
    // button { onclick: move |_| window().set_zoom_level(2.0), "Zoom 2x" }
    // button { onclick: move |_| window().set_zoom_level(3.0), "Zoom 3x" }
    rsx! {
        div { "Zoom me!" }
    }
}
