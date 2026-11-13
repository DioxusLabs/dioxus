fn app() -> Element {
    rsx! {
        div { class : "wrapper", input { oninput : move | i |
        async move { email.set(i.value()); }, r#type : "text", } button {
        onclick : move | _ | { spawn(async move { let users = get_all_users(). await .ok(); if let
        Some(users) = users { info!("{:?}", users); } }); }, "get" }
        {
            names
                .iter()
                .filter(|n| n.enabled)
                .map(|n| n.label.clone())
                .collect::<Vec<_>>()
                .join(", ")
        } }
    }
}
