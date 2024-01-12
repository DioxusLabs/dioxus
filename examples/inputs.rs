//! This example roughly shows how events are serialized into Rust from JavaScript.
//!
//! There is some conversion happening when input types are checkbox/radio/select/textarea etc.

use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(app);
}

const FIELDS: &[(&str, &str)] = &[
    ("button", "Click me!"),
    ("checkbox", "CHECKBOX"),
    ("color", ""),
    ("date", ""),
    ("datetime-local", ""),
    ("email", ""),
    ("file", ""),
    ("image", ""),
    ("number", ""),
    ("password", ""),
    ("radio", ""),
    ("range", ""),
    ("reset", ""),
    ("search", ""),
    ("submit", ""),
    ("tel", ""),
    ("text", ""),
    ("time", ""),
    ("url", ""),
    // less supported things
    ("hidden", ""),
    ("month", ""), // degrades to text most of the time, but works properly as "value'"
    ("week", ""),  // degrades to text most of the time
];

fn app(cx: Scope) -> Element {
    cx.render(rsx! {
        div { margin_left: "30px",
            {select_example(cx)},
            div {
                // handling inputs on divs will catch all input events below
                // so the value of our input event will be either huey, dewey, louie, or true/false (because of the checkboxe)
                // be mindful in grouping inputs together, as they will all be handled by the same event handler
                oninput: move |evt| {
                    println!("{evt:?}");
                },
                div {
                    input {
                        id: "huey",
                        r#type: "radio",
                        value: "huey",
                        checked: "",
                        name: "drone",
                    }
                    label {
                        r#for: "huey",
                        "Huey"
                    }
                }
                div {
                    input {
                        id: "dewey",
                        r#type: "radio",
                        value: "dewey",
                        name: "drone",
                    }
                    label {
                        r#for: "dewey",
                        "Dewey"
                    }
                }
                div {
                    input {
                        id: "louie",
                        value: "louie",
                        r#type: "radio",
                        name: "drone",
                    }
                    label {
                        r#for: "louie",
                        "Louie"
                    }
                }
                div {
                    input {
                        id: "groovy",
                        value: "groovy",
                        r#type: "checkbox",
                        name: "drone",
                    }
                    label {
                        r#for: "groovy",
                        "groovy"
                    }
                }
            }

            // elements with driven values will preventdefault automatically.
            // you can disable this override with preventdefault: false
            div {
                input {
                    id: "pdf",
                    value: "pdf",
                    name: "pdf",
                    r#type: "checkbox",
                    oninput: move |evt| {
                        println!("{evt:?}");
                    },
                }
                label {
                    r#for: "pdf",
                    "pdf"
                }
            }

            for (field, value) in FIELDS.iter() {
                div {
                    input {
                        id: "{field}",
                        name: "{field}",
                        r#type: "{field}",
                        value: "{value}",
                        oninput: move |evt: FormEvent| {
                            println!("{evt:?}");
                        },
                    }
                    label {
                        r#for: "{field}",
                        "{field} element"
                    }
                    br {}
                }
            }
        }
    })
}

fn select_example(cx: Scope) -> Element {
    cx.render(rsx! {
    div {
        select {
            id: "selection",
            name: "selection",
            multiple: true,
            oninput: move |evt| {
                println!("{evt:?}");
            },
            option {
                value : "Option 1",
                label : "Option 1",
            }
            option {
                value : "Option 2",
                label : "Option 2",
                selected : true,
            },
            option {
                value : "Option 3",
                label : "Option 3",
            }
        }
        label {
            r#for: "selection",
            "select element"
        }
    }})
}
