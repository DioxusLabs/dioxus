fn it_works() {
    rsx! {
        div {
            span { "Description: ", {package.description.as_deref().unwrap_or("❌❌❌❌ missing")} }
        }
    }
}
