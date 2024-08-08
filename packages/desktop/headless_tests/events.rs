use std::collections::HashMap;

use dioxus::html::geometry::euclid::Vector3D;
use dioxus::prelude::*;
use dioxus_desktop::DesktopContext;

#[path = "./utils.rs"]
mod utils;

pub fn main() {
    #[cfg(not(windows))]
    utils::check_app_exits(app);
}

static RECEIVED_EVENTS: GlobalSignal<usize> = Signal::global(|| 0);

fn app() -> Element {
    let desktop_context: DesktopContext = consume_context();

    let received = RECEIVED_EVENTS();
    let expected = utils::EXPECTED_EVENTS();

    use_memo(move || {
        println!("expecting {} events", utils::EXPECTED_EVENTS());
        println!("received {} events", RECEIVED_EVENTS());
    });

    if expected != 0 && received == expected {
        println!("all events received");
        desktop_context.close();
    }

    rsx! {
        div {
            test_mounted {}
            test_button {}
            test_mouse_move_div {}
            test_mouse_click_div {}
            test_mouse_dblclick_div {}
            test_mouse_down_div {}
            test_mouse_up_div {}
            test_mouse_scroll_div {}
            test_key_down_div {}
            test_key_up_div {}
            test_key_press_div {}
            test_focus_in_div {}
            test_focus_out_div {}
            test_form_input {}
            test_form_submit {}
            test_select_multiple_options {}
        }
    }
}

fn test_mounted() -> Element {
    use_hook(|| utils::EXPECTED_EVENTS.with_mut(|x| *x += 1));

    rsx! {
        div {
            width: "100px",
            height: "100px",
            onmounted: move |evt| async move {
                let rect = evt.get_client_rect().await.unwrap();
                println!("rect: {:?}", rect);
                assert_eq!(rect.width(), 100.0);
                assert_eq!(rect.height(), 100.0);
                RECEIVED_EVENTS.with_mut(|x| *x += 1);
            }
        }
    }
}

fn test_button() -> Element {
    utils::mock_event(
        "button",
        r#"new MouseEvent("click", {
            view: window,
            bubbles: true,
            cancelable: true,
            button: 0,
        })"#,
    );

    rsx! {
        button {
            id: "button",
            onclick: move |event| {
                println!("{:?}", event.data);
                assert!(event.data.modifiers().is_empty());
                assert!(event.data.held_buttons().is_empty());
                assert_eq!(
                    event.data.trigger_button(),
                    Some(dioxus_html::input_data::MouseButton::Primary),
                );
                RECEIVED_EVENTS.with_mut(|x| *x += 1);
            }
        }
    }
}

fn test_mouse_move_div() -> Element {
    utils::mock_event(
        "mouse_move_div",
        r#"new MouseEvent("mousemove", {
        view: window,
        bubbles: true,
        cancelable: true,
        buttons: 2,
        })"#,
    );

    rsx! {
        div {
            id: "mouse_move_div",
            onmousemove: move |event| {
                println!("{:?}", event.data);
                assert!(event.data.modifiers().is_empty());
                assert!(
                    event
                        .data
                        .held_buttons()
                        .contains(dioxus_html::input_data::MouseButton::Secondary),
                );
                RECEIVED_EVENTS.with_mut(|x| *x += 1);
            }
        }
    }
}

fn test_mouse_click_div() -> Element {
    utils::mock_event(
        "mouse_click_div",
        r#"new MouseEvent("click", {
        view: window,
        bubbles: true,
        cancelable: true,
        buttons: 2,
        button: 2,
        })"#,
    );

    rsx! {
        div {
            id: "mouse_click_div",
            onclick: move |event| {
                println!("{:?}", event.data);
                assert!(event.data.modifiers().is_empty());
                assert!(
                    event
                        .data
                        .held_buttons()
                        .contains(dioxus_html::input_data::MouseButton::Secondary),
                );
                assert_eq!(
                    event.data.trigger_button(),
                    Some(dioxus_html::input_data::MouseButton::Secondary),
                );
                RECEIVED_EVENTS.with_mut(|x| *x += 1);
            }
        }
    }
}

fn test_mouse_dblclick_div() -> Element {
    utils::mock_event(
        "mouse_dblclick_div",
        r#"new MouseEvent("dblclick", {
        view: window,
        bubbles: true,
        cancelable: true,
        buttons: 1|2,
        button: 2,
        })"#,
    );

    rsx! {
        div {
            id: "mouse_dblclick_div",
            ondoubleclick: move |event| {
                println!("{:?}", event.data);
                assert!(event.data.modifiers().is_empty());
                assert!(
                    event
                        .data
                        .held_buttons()
                        .contains(dioxus_html::input_data::MouseButton::Primary),
                );
                assert!(
                    event
                        .data
                        .held_buttons()
                        .contains(dioxus_html::input_data::MouseButton::Secondary),
                );
                assert_eq!(
                    event.data.trigger_button(),
                    Some(dioxus_html::input_data::MouseButton::Secondary),
                );
                RECEIVED_EVENTS.with_mut(|x| *x += 1);
            }
        }
    }
}

fn test_mouse_down_div() -> Element {
    utils::mock_event(
        "mouse_down_div",
        r#"new MouseEvent("mousedown", {
        view: window,
        bubbles: true,
        cancelable: true,
        buttons: 2,
        button: 2,
        })"#,
    );

    rsx! {
        div {
            id: "mouse_down_div",
            onmousedown: move |event| {
                println!("{:?}", event.data);
                assert!(event.data.modifiers().is_empty());
                assert!(
                    event
                        .data
                        .held_buttons()
                        .contains(dioxus_html::input_data::MouseButton::Secondary),
                );
                assert_eq!(
                    event.data.trigger_button(),
                    Some(dioxus_html::input_data::MouseButton::Secondary),
                );
                RECEIVED_EVENTS.with_mut(|x| *x += 1);
            }
        }
    }
}

fn test_mouse_up_div() -> Element {
    utils::mock_event(
        "mouse_up_div",
        r#"new MouseEvent("mouseup", {
        view: window,
        bubbles: true,
        cancelable: true,
        buttons: 0,
        button: 0,
        })"#,
    );

    rsx! {
        div {
            id: "mouse_up_div",
            onmouseup: move |event| {
                println!("{:?}", event.data);
                assert!(event.data.modifiers().is_empty());
                assert!(event.data.held_buttons().is_empty());
                assert_eq!(
                    event.data.trigger_button(),
                    Some(dioxus_html::input_data::MouseButton::Primary),
                );
                RECEIVED_EVENTS.with_mut(|x| *x += 1);
            }
        }
    }
}

fn test_mouse_scroll_div() -> Element {
    utils::mock_event(
        "wheel_div",
        r#"new WheelEvent("wheel", {
        view: window,
        deltaX: 1.0,
        deltaY: 2.0,
        deltaZ: 3.0,
        deltaMode: 0x00,
        bubbles: true,
        })"#,
    );

    rsx! {
        div {
            id: "wheel_div",
            width: "100px",
            height: "100px",
            background_color: "red",
            onwheel: move |event| {
                println!("{:?}", event.data);
                let dioxus_html::geometry::WheelDelta::Pixels(delta) = event.data.delta() else {
                    panic!("Expected delta to be in pixels")
                };
                assert_eq!(delta, Vector3D::new(1.0, 2.0, 3.0));
                RECEIVED_EVENTS.with_mut(|x| *x += 1);
            }
        }
    }
}

fn test_key_down_div() -> Element {
    utils::mock_event(
        "key_down_div",
        r#"new KeyboardEvent("keydown", {
        key: "a",
        code: "KeyA",
        location: 0,
        repeat: true,
        keyCode: 65,
        charCode: 97,
        char: "a",
        charCode: 0,
        altKey: false,
        ctrlKey: false,
        metaKey: false,
        shiftKey: false,
        isComposing: true,
        which: 65,
        bubbles: true,
        })"#,
    );
    rsx! {
        input {
            id: "key_down_div",
            onkeydown: move |event| {
                println!("{:?}", event.data);
                assert!(event.data.modifiers().is_empty());
                assert_eq!(event.data.key().to_string(), "a");
                assert_eq!(event.data.code().to_string(), "KeyA");
                assert_eq!(event.data.location(), Location::Standard);
                assert!(event.data.is_auto_repeating());
                assert!(event.data.is_composing());
                RECEIVED_EVENTS.with_mut(|x| *x += 1);
            }
        }
    }
}
fn test_key_up_div() -> Element {
    utils::mock_event(
        "key_up_div",
        r#"new KeyboardEvent("keyup", {
        key: "a",
        code: "KeyA",
        location: 0,
        repeat: false,
        keyCode: 65,
        charCode: 97,
        char: "a",
        charCode: 0,
        altKey: false,
        ctrlKey: false,
        metaKey: false,
        shiftKey: false,
        isComposing: false,
        which: 65,
        bubbles: true,
        })"#,
    );

    rsx! {
        input {
            id: "key_up_div",
            onkeyup: move |event| {
                println!("{:?}", event.data);
                assert!(event.data.modifiers().is_empty());
                assert_eq!(event.data.key().to_string(), "a");
                assert_eq!(event.data.code().to_string(), "KeyA");
                assert_eq!(event.data.location(), Location::Standard);
                assert!(!event.data.is_auto_repeating());
                assert!(!event.data.is_composing());
                RECEIVED_EVENTS.with_mut(|x| *x += 1);
            }
        }
    }
}
fn test_key_press_div() -> Element {
    utils::mock_event(
        "key_press_div",
        r#"new KeyboardEvent("keypress", {
        key: "a",
        code: "KeyA",
        location: 0,
        repeat: false,
        keyCode: 65,
        charCode: 97,
        char: "a",
        charCode: 0,
        altKey: false,
        ctrlKey: false,
        metaKey: false,
        shiftKey: false,
        isComposing: false,
        which: 65,
        bubbles: true,
        })"#,
    );
    rsx! {
        input {
            id: "key_press_div",
            onkeypress: move |event| {
                println!("{:?}", event.data);
                assert!(event.data.modifiers().is_empty());
                assert_eq!(event.data.key().to_string(), "a");
                assert_eq!(event.data.code().to_string(), "KeyA");
                assert_eq!(event.data.location(), Location::Standard);
                assert!(!event.data.is_auto_repeating());
                assert!(!event.data.is_composing());
                RECEIVED_EVENTS.with_mut(|x| *x += 1);
            }
        }
    }
}

fn test_focus_in_div() -> Element {
    utils::mock_event(
        "focus_in_div",
        r#"new FocusEvent("focusin", {bubbles: true})"#,
    );

    rsx! {
        input {
            id: "focus_in_div",
            onfocusin: move |event| {
                println!("{:?}", event.data);
                RECEIVED_EVENTS.with_mut(|x| *x += 1);
            }
        }
    }
}

fn test_focus_out_div() -> Element {
    utils::mock_event(
        "focus_out_div",
        r#"new FocusEvent("focusout",{bubbles: true})"#,
    );
    rsx! {
        input {
            id: "focus_out_div",
            onfocusout: move |event| {
                println!("{:?}", event.data);
                RECEIVED_EVENTS.with_mut(|x| *x += 1);
            }
        }
    }
}

fn test_form_input() -> Element {
    let mut values = use_signal(HashMap::new);

    utils::mock_event_with_extra(
        "form-username",
        r#"new Event("input", { bubbles: true, cancelable: true, composed: true })"#,
        r#"element.value = "hello";"#,
    );

    let set_username = move |ev: FormEvent| {
        values.set(ev.values());

        // The value of the input should match
        assert_eq!(ev.value(), "hello");

        // And then the value the form gives us should also match
        values.with_mut(|x| {
            assert_eq!(x.get("username").unwrap(), "hello");
            assert_eq!(x.get("full-name").unwrap(), "lorem");
            assert_eq!(x.get("password").unwrap(), "ipsum");
            assert_eq!(x.get("color").unwrap(), "red");
        });
        RECEIVED_EVENTS.with_mut(|x| *x += 1);
    };

    rsx! {
        div {
            h1 { "Form" }
            form {
                id: "form",
                oninput: move |ev| {
                    values.set(ev.values());
                },
                onsubmit: move |ev| {
                    println!("{:?}", ev);
                },
                input {
                    r#type: "text",
                    name: "username",
                    id: "form-username",
                    oninput: set_username
                }
                input { r#type: "text", name: "full-name", value: "lorem" }
                input { r#type: "password", name: "password", value: "ipsum" }
                input {
                    r#type: "radio",
                    name: "color",
                    value: "red",
                    checked: true
                }
                input { r#type: "radio", name: "color", value: "blue" }
                button { r#type: "submit", value: "Submit", "Submit the form" }
            }
        }
    }
}

fn test_form_submit() -> Element {
    let mut values = use_signal(HashMap::new);

    utils::mock_event_with_extra(
        "form-submitter",
        r#"new Event("submit", { bubbles: true, cancelable: true, composed: true })"#,
        r#"element.submit();"#,
    );

    let set_values = move |ev: FormEvent| {
        values.set(ev.values());
        values.with_mut(|x| {
            assert_eq!(x.get("username").unwrap(), "goodbye");
            assert_eq!(x.get("full-name").unwrap(), "lorem");
            assert_eq!(x.get("password").unwrap(), "ipsum");
            assert_eq!(x.get("color").unwrap(), "red");
        });
        RECEIVED_EVENTS.with_mut(|x| *x += 1);
    };

    rsx! {
        div {
            h1 { "Form" }
            form { id: "form-submitter", onsubmit: set_values,
                input {
                    r#type: "text",
                    name: "username",
                    id: "username",
                    value: "goodbye"
                }
                input { r#type: "text", name: "full-name", value: "lorem" }
                input { r#type: "password", name: "password", value: "ipsum" }
                input {
                    r#type: "radio",
                    name: "color",
                    value: "red",
                    checked: true
                }
                input { r#type: "radio", name: "color", value: "blue" }
                button { r#type: "submit", value: "Submit", "Submit the form" }
            }
        }
    }
}

fn test_select_multiple_options() -> Element {
    utils::mock_event_with_extra(
        "select-many",
        r#"new Event("input", { bubbles: true, cancelable: true, composed: true })"#,
        r#"
            document.getElementById('usa').selected = true;
            document.getElementById('canada').selected = true;
            document.getElementById('mexico').selected = false;
        "#,
    );

    rsx! {
        select {
            id: "select-many",
            name: "country",
            multiple: true,
            oninput: move |ev| {
                let values = ev.value();
                let values = values.split(',').collect::<Vec<_>>();
                assert_eq!(values, vec!["usa", "canada"]);
                RECEIVED_EVENTS.with_mut(|x| *x += 1);
            },
            option { id: "usa", value: "usa", "USA" }
            option { id: "canada", value: "canada", "Canada" }
            option { id: "mexico", value: "mexico", selected: true, "Mexico" }
        }
    }
}
