fn main() {
    rsx! {
        div {
            {
                let millis = timer.with(|t| t.duration().saturating_sub(t.started_at.map(|x| x.elapsed()).unwrap_or(Duration::ZERO)).as_millis());
                format!("{:02}:{:02}:{:02}.{:01}",
                        millis / 1000 / 3600 % 3600,
                        millis / 1000 / 60 % 60,
                        millis / 1000 % 60,
                        millis / 100 % 10)
            }
        }
        div {
            input {
                r#type: "number",
                min: 0,
                max: 99,
                value: format!("{:02}", timer.read().hours),
                oninput: move |e| {
                    timer.write().hours = e.value().parse().unwrap_or(0);
                }
            }
            // some comment
            input {
                r#type: "number",
                min: 0,
                max: 99,
                value: format!("{:02}", timer.read().hours),
                oninput: move |e| {
                    timer.write().hours = e.value().parse().unwrap_or(0);
                }
            }
        }
    }
}
