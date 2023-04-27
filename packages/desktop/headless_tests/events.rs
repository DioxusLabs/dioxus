use crate::check_app_exits;
use dioxus::prelude::*;
use dioxus_desktop::DesktopContext;
use dioxus::html::geometry::euclid::Vector3D;

pub fn test_events() {
    check_app_exits(app);
}

fn mock_event(cx: &ScopeState, id: &'static str, value: &'static str) {
    use_effect(cx, (), |_| {
        let desktop_context: DesktopContext = cx.consume_context().unwrap();
        async move {
            desktop_context.eval(&format!(
                r#"let element = document.getElementById('{}');
                // Dispatch a synthetic event
                const event = {};
                element.dispatchEvent(event);
                "#,
                id, value
            ));
        }
    });
}

#[allow(deprecated)]
fn app(cx: Scope) -> Element {
    let desktop_context: DesktopContext = cx.consume_context().unwrap();
    let recieved_events = use_state(cx, || 0);

    // button
    mock_event(
        &cx,
        "button",
        r#"new MouseEvent("click", {
    view: window,
    bubbles: true,
    cancelable: true,
    button: 0,
  })"#,
    );
    // mouse_move_div
    mock_event(
        &cx,
        "mouse_move_div",
        r#"new MouseEvent("mousemove", {
    view: window,
    bubbles: true,
    cancelable: true,
    buttons: 2,
    })"#,
        );
    // mouse_click_div
    mock_event(
        &cx,
        "mouse_click_div",
        r#"new MouseEvent("click", {
    view: window,
    bubbles: true,
    cancelable: true,
    buttons: 2,
    button: 2,
    })"#
    );
    // mouse_dblclick_div
    mock_event(
        &cx,
        "mouse_dblclick_div",
        r#"new MouseEvent("dblclick", {
    view: window,
    bubbles: true,
    cancelable: true,
    buttons: 1|2,
    button: 2,
    })"#,
    );
    // mouse_down_div
    mock_event(
        &cx,
        "mouse_down_div",
        r#"new MouseEvent("mousedown", {
    view: window,
    bubbles: true,
    cancelable: true,
    buttons: 2,
    button: 2,
    })"#,
    );
    // mouse_up_div
    mock_event(
        &cx,
        "mouse_up_div",
        r#"new MouseEvent("mouseup", {
    view: window,
    bubbles: true,
    cancelable: true,
    buttons: 0,
    button: 0,
    })"#,
    );
    // wheel_div
    mock_event(
        &cx,
        "wheel_div",
        r#"new WheelEvent("wheel", {
    deltaX: 1.0,
    deltaY: 2.0,
    deltaZ: 3.0,
    })"#,
    );
    // key_down_div
    mock_event(
        &cx,
        "key_down_div",
        r#"new KeyboardEvent("keydown", {
    key: "a",
    code: "KeyA",
    location: 0,
    repeat: true,
    })"#,
    );
    // key_up_div
    mock_event(
        &cx,
        "key_up_div",
        r#"new KeyboardEvent("keyup", {
    key: "a",
    code: "KeyA",
    location: 0,
    repeat: false,
    })"#,
    );
    // key_press_div
    mock_event(
        &cx,
        "key_press_div",
        r#"new KeyboardEvent("keypress", {
    key: "a",
    code: "KeyA",
    location: 0,
    repeat: false,
    })"#,
    );
    // focus_in_div
    mock_event(
        &cx,
        "focus_in_div",
        r#"new FocusEvent("focusin")"#,
    );
    // focus_out_div
    mock_event(
        &cx,
        "focus_out_div",
        r#"new FocusEvent("focusout")"#,
    );
    

    if **recieved_events == 12 {
        desktop_context.close();
    }

    cx.render(rsx! {
        div {
            button {
                id: "button",
                onclick: move |event| {
                    println!("{:?}", event.data);
                    assert!(event.data.modifiers().is_empty());
                    assert!(event.data.held_buttons().is_empty());
                    assert_eq!(event.data.trigger_button(), Some(dioxus_html::input_data::MouseButton::Primary));
                    recieved_events.modify(|x| *x + 1)
                },
            }
            div {
                id: "mouse_move_div",
                onmousemove: move |event| {
                    println!("{:?}", event.data);
                    assert!(event.data.modifiers().is_empty());
                    assert!(event.data.held_buttons().contains(dioxus_html::input_data::MouseButton::Secondary));
                    recieved_events.modify(|x| *x + 1)
                },
            }
            div {
                id: "mouse_click_div",
                onclick: move |event| {
                    println!("{:?}", event.data);
                    assert!(event.data.modifiers().is_empty());
                    assert!(event.data.held_buttons().contains(dioxus_html::input_data::MouseButton::Secondary));
                    assert_eq!(event.data.trigger_button(), Some(dioxus_html::input_data::MouseButton::Secondary));
                    recieved_events.modify(|x| *x + 1)
                },
            }
            div{
                id: "mouse_dblclick_div",
                ondblclick: move |event| {
                    println!("{:?}", event.data);
                    assert!(event.data.modifiers().is_empty());
                    assert!(event.data.held_buttons().contains(dioxus_html::input_data::MouseButton::Primary));
                    assert!(event.data.held_buttons().contains(dioxus_html::input_data::MouseButton::Secondary));
                    assert_eq!(event.data.trigger_button(), Some(dioxus_html::input_data::MouseButton::Secondary));
                    recieved_events.modify(|x| *x + 1)
                }
            }
            div{
                id: "mouse_down_div",
                onmousedown: move |event| {
                    println!("{:?}", event.data);
                    assert!(event.data.modifiers().is_empty());
                    assert!(event.data.held_buttons().contains(dioxus_html::input_data::MouseButton::Secondary));
                    assert_eq!(event.data.trigger_button(), Some(dioxus_html::input_data::MouseButton::Secondary));
                    recieved_events.modify(|x| *x + 1)
                }
            }
            div{
                id: "mouse_up_div",
                onmouseup: move |event| {
                    println!("{:?}", event.data);
                    assert!(event.data.modifiers().is_empty());
                    assert!(event.data.held_buttons().is_empty());
                    assert_eq!(event.data.trigger_button(), Some(dioxus_html::input_data::MouseButton::Primary));
                    recieved_events.modify(|x| *x + 1)
                }
            }
            div{
                id: "wheel_div",
                onwheel: move |event| {
                    println!("{:?}", event.data);
                    let dioxus_html::geometry::WheelDelta::Pixels(delta)= event.data.delta()else{
                        panic!("Expected delta to be in pixels")
                    };
                    assert_eq!(delta, Vector3D::new(1.0, 2.0, 3.0));
                    recieved_events.modify(|x| *x + 1)
                }
            }
            div{
                id: "key_down_div",
                onkeydown: move |event| {
                    println!("{:?}", event.data);
                    assert!(event.data.modifiers().is_empty());
                    assert_eq!(event.data.key().to_string(), "a");
                    assert_eq!(event.data.code().to_string(), "KeyA");
                    assert_eq!(event.data.location, 0);
                    assert!(event.data.is_auto_repeating());
                    
                    recieved_events.modify(|x| *x + 1)
                }
            }
            div{
                id: "key_up_div",
                onkeyup: move |event| {
                    println!("{:?}", event.data);
                    assert!(event.data.modifiers().is_empty());
                    assert_eq!(event.data.key().to_string(), "a");
                    assert_eq!(event.data.code().to_string(), "KeyA");
                    assert_eq!(event.data.location, 0);
                    assert!(!event.data.is_auto_repeating());

                    recieved_events.modify(|x| *x + 1)
                }
            }
            div{
                id: "key_press_div",
                onkeypress: move |event| {
                    println!("{:?}", event.data);
                    assert!(event.data.modifiers().is_empty());
                    assert_eq!(event.data.key().to_string(), "a");
                    assert_eq!(event.data.code().to_string(), "KeyA");
                    assert_eq!(event.data.location, 0);
                    assert!(!event.data.is_auto_repeating());
                    
                    recieved_events.modify(|x| *x + 1)
                }
            }
            div{
                id: "focus_in_div",
                onfocusin: move |event| {
                    println!("{:?}", event.data);
                    recieved_events.modify(|x| *x + 1)
                }
            }
            div{
                id: "focus_out_div",
                onfocusout: move |event| {
                    println!("{:?}", event.data);
                    recieved_events.modify(|x| *x + 1)
                }
            }
        }
    })
}
