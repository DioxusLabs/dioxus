rsx! {
    for (index, name, value) in data.iter() {
        div { "{index}: {name} = {value}" }
    }

    for item in items {
        div { "{item}" }
    }

    for (a, b) in pairs.iter().enumerate() {
        span { "{a}-{b}" }
    }

    for (_, value) in pairs {
        span { "{value}" }
    }
}
