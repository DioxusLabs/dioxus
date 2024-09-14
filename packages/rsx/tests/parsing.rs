use dioxus_rsx::CallBody;
use quote::ToTokens;

use prettier_please::PrettyUnparse;

#[test]
fn callbody_ctx() {
    let item = quote::quote! {
        div {
            h1 {
                id: "Some cool attribute {cool}"
            }
            for item in items {
                "Something {cool}"
            }
            Component {
                "Something {elseish}"
            }
            Component2 {
                "Something {Body}"
                Component3 {
                    prop: "Something {Body3}",
                    "Something {Body4}"
                }
            }
            something-cool {
                "Something {cool}ish"
            }
            if true {
                div {
                    "hi! {cool}"
                }
            }
            "hi!"
            "goodbye!"
        }
        {some_expr}
    };

    let cb: CallBody = syn::parse2(item).unwrap();

    dbg!(cb.template_idx.get());
}

#[test]
fn simple_case() {
    let item = quote::quote! {
        div {
            something: "cool",
            id: "Some cool attribute {cool}",
            class: "Some cool attribute {cool2}",
            "hi!"
            {some_expr}
            Component {
                boolish: true,
                otherish: 123,
                otherish2: 123.0,
                otherish3: "dang!",
                otherish3: "dang! {cool}",
            }
        }
    };

    let cb: CallBody = syn::parse2(item).unwrap();
    println!("{}", cb.to_token_stream().pretty_unparse());
}

#[test]
fn complex_kitchen_sink() {
    let item = quote::quote! {
        // complex_carry
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
            div { class: "px-4", {is_current.then(|| rsx! { children })} }
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

        for i in 0..10 {
            Component { key: "{i}", blah: 120 }
        }
        for i in 0..10 {
            Component { key: "{i}" }
        }

        for i in 0..10 {
            div { key: "{i}", blah: 120 }
        }

        for i in 0..10 {
            div { key: "{i}" }
        }

        div {
            "asdbascasdbasd"
            "asbdasbdabsd"
            {asbdabsdbasdbas}
        }
    };

    let _cb: CallBody = syn::parse2(item).unwrap();
}
