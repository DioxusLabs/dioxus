fn main() {
    rsx! {
        button {
            id: "start_stop",
            onclick: move |_| timer.with_mut(|t| t.started_at = if t.started_at.is_none() { Some(Instant::now()) } else { None } )
        }
    }
}
