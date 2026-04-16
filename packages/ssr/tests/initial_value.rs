use dioxus::prelude::*;

#[test]
fn initial_value_renders_as_value() {
    assert_eq!(
        dioxus_ssr::render_element(rsx! {
            input { initial_value: "hello" }
        }),
        r#"<input value="hello"/>"#
    );
}

#[test]
fn initial_checked_renders_as_checked() {
    assert_eq!(
        dioxus_ssr::render_element(rsx! {
            input { r#type: "checkbox", initial_checked: true }
        }),
        r#"<input type="checkbox" checked=true/>"#
    );
}

#[test]
fn initial_selected_renders_as_selected() {
    assert_eq!(
        dioxus_ssr::render_element(rsx! {
            option { initial_selected: true }
        }),
        r#"<option selected=true></option>"#
    );
}

#[test]
fn dynamic_initial_value() {
    fn app() -> Element {
        let value = "dynamic";
        rsx! {
            input { initial_value: value }
        }
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild(&mut dioxus_core::NoOpMutations);

    assert_eq!(dioxus_ssr::render(&dom), r#"<input value="dynamic"/>"#);
}
