fn it_works() {
    cx.render(rsx! {
        div {
            span { "Description: ", {package.description.as_deref().unwrap_or("❌❌❌❌ missing")} }
        }
    })
}
