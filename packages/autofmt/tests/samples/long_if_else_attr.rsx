rsx! {
    button { class:
        if is_active {
            "w-full text-left px-3 py-1 text-xs font-mono text-primary bg-select"
        } else {
            "w-full text-left px-3 py-1 text-xs font-mono text-secondary hover:bg-select hover:text-primary"
        },
        "Click me"
    }
}
