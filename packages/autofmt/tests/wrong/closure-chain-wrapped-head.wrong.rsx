#[component]
fn App() -> Element {
    rsx! {
        input {
            onchange: move |_| {
                let roster: Vec<(String, String)> =
                    crate::roster::parse_member_roster(bytes.to_vec())
                        .into_iter()
                        .map(|e| (e.name, e.email))
                        .collect();
            },
        }
    }
}
