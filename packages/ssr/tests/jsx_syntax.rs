#![allow(non_snake_case)]

use dioxus::prelude::*;

#[test]
fn jsx_elements() {
    let name = "world";
    assert_eq!(
        dioxus_ssr::render_element(rsx! {
            <div class="container">
                <h1>"Hello, {name}!"</h1>
                <img src="image.png" />
            </div>
        }),
        r#"<div class="container"><h1>Hello, world!</h1><img src="image.png"/></div>"#
    );
}

#[test]
fn jsx_components() {
    #[component]
    fn Wrapper(title: String, children: Element) -> Element {
        rsx! {
            <section>
                <h2>"{title}"</h2>
                {children}
            </section>
        }
    }

    assert_eq!(
        dioxus_ssr::render_element(rsx! {
            <Wrapper title="hi">
                <p>"body"</p>
            </Wrapper>
        }),
        "<section><h2>hi</h2><p>body</p></section>"
    );
}

#[test]
fn jsx_mixed_with_regular_syntax() {
    let count = 3;
    assert_eq!(
        dioxus_ssr::render_element(rsx! {
            div { class: "outer",
                <span>"jsx child"</span>
                p { "regular child" }
                for i in 0..count {
                    <b>"{i}"</b>
                }
                if count > 1 {
                    <i>"many"</i>
                }
            }
        }),
        r#"<div class="outer"><span>jsx child</span><p>regular child</p><b>0</b><b>1</b><b>2</b><i>many</i></div>"#
    );
}

#[test]
fn jsx_event_handlers_and_shorthand() {
    fn App() -> Element {
        let disabled = true;
        rsx! {
            <button disabled onclick={move |_| println!("clicked")}>"Click me"</button>
        }
    }

    let mut dom = VirtualDom::new(App);
    dom.rebuild(&mut dioxus_core::NoOpMutations);

    assert_eq!(
        dioxus_ssr::render(&dom),
        "<button disabled=true>Click me</button>"
    );
}
