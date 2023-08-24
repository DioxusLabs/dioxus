use dioxus::html::geometry::euclid::Vector3D;
use dioxus::prelude::*;
use dioxus_desktop::DesktopContext;

pub(crate) fn check_app_exits(app: Component) {
    use dioxus_desktop::tao::window::WindowBuilder;
    use dioxus_desktop::Config;
    // This is a deadman's switch to ensure that the app exits
    let should_panic = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let should_panic_clone = should_panic.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(100));
        if should_panic_clone.load(std::sync::atomic::Ordering::SeqCst) {
            std::process::exit(exitcode::SOFTWARE);
        }
    });

    dioxus_desktop::launch_cfg(
        app,
        Config::new().with_window(WindowBuilder::new().with_visible(false)),
    );

    // Stop deadman's switch
    should_panic.store(false, std::sync::atomic::Ordering::SeqCst);
}

pub fn main() {
    check_app_exits(app);
}

fn mock_event(cx: &ScopeState, id: &'static str, value: &'static str) {
    let eval_provider = use_eval(cx).clone();

    use_effect(cx, (), move |_| async move {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let js = format!(
            r#"
                //console.log("ran");
                // Dispatch a synthetic event
                let event = {};
                let element = document.getElementById('{}');
                console.log(element, event);
                element.dispatchEvent(event);
                "#,
            value, id
        );

        eval_provider(&js).unwrap();
    })
}

#[allow(deprecated)]
fn app(cx: Scope) -> Element {
    let desktop_context: DesktopContext = cx.consume_context().unwrap();
    let recieved_events = use_state(cx, || 0);

    // button
    mock_event(
        cx,
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
        cx,
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
        cx,
        "mouse_click_div",
        r#"new MouseEvent("click", {
        view: window,
        bubbles: true,
        cancelable: true,
        buttons: 2,
        button: 2,
        })"#,
    );
    // mouse_dblclick_div
    mock_event(
        cx,
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
        cx,
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
        cx,
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
        cx,
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
    // key_down_div
    mock_event(
        cx,
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
        isComposing: false,
        which: 65,
        bubbles: true,
        })"#,
    );
    // key_up_div
    mock_event(
        cx,
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
    // key_press_div
    mock_event(
        cx,
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
    // focus_in_div
    mock_event(
        cx,
        "focus_in_div",
        r#"new FocusEvent("focusin", {bubbles: true})"#,
    );
    // focus_out_div
    mock_event(
        cx,
        "focus_out_div",
        r#"new FocusEvent("focusout",{bubbles: true})"#,
    );

    if **recieved_events == 12 {
        println!("all events recieved");
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
                width: "100px",
                height: "100px",
                background_color: "red",
                onwheel: move |event| {
                    println!("{:?}", event.data);
                    let dioxus_html::geometry::WheelDelta::Pixels(delta)= event.data.delta()else{
                        panic!("Expected delta to be in pixels")
                    };
                    assert_eq!(delta, Vector3D::new(1.0, 2.0, 3.0));
                    recieved_events.modify(|x| *x + 1)
                }
            }
            input{
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
            input{
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
            input{
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
            input{
                id: "focus_in_div",
                onfocusin: move |event| {
                    println!("{:?}", event.data);
                    recieved_events.modify(|x| *x + 1)
                }
            }
            input{
                id: "focus_out_div",
                onfocusout: move |event| {
                    println!("{:?}", event.data);
                    recieved_events.modify(|x| *x + 1)
                }
            }
        }
    })
}
