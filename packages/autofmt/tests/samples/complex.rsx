rsx! {
    // Complex nesting with components
    button {
        class: "flex items-center pl-3 py-3 pr-2 text-gray-500 hover:bg-indigo-50 rounded",
        onclick: move |evt| {
            show_user_menu.set(!show_user_menu.get());
            evt.cancel_bubble();
        },
        onmousedown: move |evt| show_user_menu.set(!show_user_menu.get()),
        span { class: "inline-block mr-4", icons::icon_14 {} }
        span { "Settings" }
    }

    // Complex nesting with handlers
    li {
        Link {
            class: "flex items-center pl-3 py-3 pr-4 {active_class} rounded",
            to: "{to}",
            span { class: "inline-block mr-3", icons::icon_0 {} }
            span { "{name}" }
            {children.is_some().then(|| rsx! {
                span {
                    class: "inline-block ml-auto hover:bg-gray-500",
                    onclick: move |evt| {
                        // open.set(!open.get());
                        evt.cancel_bubble();
                    },
                    icons::icon_8 {}
                }
            })}
        }
        div { class: "px-4", {is_current.then(|| rsx!{ children })} }
    }

    // No nesting
    Component {
        adsasd: "asd",
        onclick: move |_| {
            let blah = 120;
        }
    }

    // Component path
    my::thing::Component {
        adsasd: "asd",
        onclick: move |_| {
            let blah = 120;
        }
    }

    div { "asdbascasdbasd", "asbdasbdabsd", {asbdabsdbasdbas} }
}
