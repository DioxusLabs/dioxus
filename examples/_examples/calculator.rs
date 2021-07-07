//! Example: Calculator
//! ------------------
//!
//! This example showcases a basic iOS-style calculator app using classic react-style `use_state` hooks. A counterpart of
//! this example is in `model.rs` which shows the same calculator app implemented using the `model` paradigm. We've found
//! that the most natural Rust code is more "model-y" than "React-y", but opt to keep this example to show the different
//! flavors of programming you can use in Dioxus.

use dioxus::events::on::*;
use dioxus::prelude::*;
use dioxus_core as dioxus;
use dioxus_html_namespace as dioxus_elements;
use dioxus_web::WebsysRenderer;

const STYLE: &str = include_str!("../../../examples/assets/calculator.css");

fn main() {
    wasm_logger::init(wasm_logger::Config::new(log::Level::Debug));
    console_error_panic_hook::set_once();
    wasm_bindgen_futures::spawn_local(WebsysRenderer::start(App));
}

enum Operator {
    Add,
    Sub,
    Mul,
    Div,
}

static App: FC<()> = |cx| {
    let (cur_val, set_cur_val) = use_state_classic(cx, || 0.0_f64);
    let (operator, set_operator) = use_state_classic(cx, || None as Option<Operator>);
    let (display_value, set_display_value) = use_state_classic(cx, || "0".to_string());

    let clear_display = display_value.eq("0");
    let clear_text = if clear_display { "C" } else { "AC" };

    let input_digit = move |num: u8| {
        let mut new = if operator.is_some() {
            String::new()
        } else if display_value == "0" {
            String::new()
        } else {
            display_value.clone()
        };
        if operator.is_some() {
            let val = display_value.parse::<f64>().unwrap();
            set_cur_val(val);
        }

        new.push_str(num.to_string().as_str());
        set_display_value(new);
    };

    let input_dot = move || {
        let mut new = display_value.clone();
        new.push_str(".");
        set_display_value(new);
    };

    let perform_operation = move || {
        if let Some(op) = operator.as_ref() {
            let rhs = display_value.parse::<f64>().unwrap();
            let new_val = match op {
                Operator::Add => *cur_val + rhs,
                Operator::Sub => *cur_val - rhs,
                Operator::Mul => *cur_val * rhs,
                Operator::Div => *cur_val / rhs,
            };
            set_cur_val(new_val);
            set_display_value(new_val.to_string());
            set_operator(None);
        }
    };

    let toggle_sign = move |_| {
        if display_value.starts_with("-") {
            set_display_value(display_value.trim_start_matches("-").to_string())
        } else {
            set_display_value(format!("-{}", *display_value))
        }
    };
    let toggle_percent = move |_| todo!();

    let clear_key = move |_| {
        set_display_value("0".to_string());
        if !clear_display {
            set_operator(None);
            set_cur_val(0.0);
        }
    };

    let keydownhandler = move |evt: KeyboardEvent| match evt.key_code() {
        KeyCode::Backspace => {
            let mut new = display_value.clone();
            if !new.as_str().eq("0") {
                new.pop();
            }
            set_display_value(new);
        }
        KeyCode::_0 => input_digit(0),
        KeyCode::_1 => input_digit(1),
        KeyCode::_2 => input_digit(2),
        KeyCode::_3 => input_digit(3),
        KeyCode::_4 => input_digit(4),
        KeyCode::_5 => input_digit(5),
        KeyCode::_6 => input_digit(6),
        KeyCode::_7 => input_digit(7),
        KeyCode::_8 => input_digit(8),
        KeyCode::_9 => input_digit(9),
        KeyCode::Add => set_operator(Some(Operator::Add)),
        KeyCode::Subtract => set_operator(Some(Operator::Sub)),
        KeyCode::Divide => set_operator(Some(Operator::Div)),
        KeyCode::Multiply => set_operator(Some(Operator::Mul)),
        _ => {}
    };

    cx.render(rsx! {
        div {
            id: "wrapper"
            div { class: "app" onkeydown: {keydownhandler}
                style { "{STYLE}" }
                div { class: "calculator", 
                    CalculatorDisplay { val: &display_value}
                    div { class: "calculator-keypad"
                        div { class: "input-keys"
                            div { class: "function-keys"
                                CalculatorKey { name: "key-clear", onclick: {clear_key} "{clear_text}" }
                                CalculatorKey { name: "key-sign", onclick: {toggle_sign}, "±"}
                                CalculatorKey { name: "key-percent", onclick: {toggle_percent} "%"}
                            }
                            div { class: "digit-keys"
                                CalculatorKey { name: "key-0", onclick: move |_| input_digit(0), "0" }
                                CalculatorKey { name: "key-dot", onclick: move |_|  input_dot(), "●" }
                                {(1..10).map(move |k| rsx!{
                                    CalculatorKey { key: "{k}", name: "key-{k}", onclick: move |_| input_digit(k), "{k}" }
                                })}
                            }
                        }
                        div { class: "operator-keys"
                            CalculatorKey { name: "key-divide", onclick: move |_| set_operator(Some(Operator::Div)) "÷" }
                            CalculatorKey { name: "key-multiply", onclick: move |_| set_operator(Some(Operator::Mul)) "×" }
                            CalculatorKey { name: "key-subtract", onclick: move |_| set_operator(Some(Operator::Sub)) "−" }
                            CalculatorKey { name: "key-add", onclick: move |_| set_operator(Some(Operator::Add)) "+" }
                            CalculatorKey { name: "key-equals", onclick: move |_| perform_operation() "=" }
                        }
                    }
                }
            }
        }
    })
};

#[derive(Props)]
struct CalculatorKeyProps<'a> {
    /// Name!
    name: &'static str,

    /// Click!
    onclick: &'a dyn Fn(MouseEvent),
}

fn CalculatorKey<'a, 'r>(cx: Context<'a, CalculatorKeyProps<'r>>) -> VNode<'a> {
    cx.render(rsx! {
        button {
            class: "calculator-key {cx.name}"
            onclick: {cx.onclick}
            {cx.children()}
        }
    })
}

#[derive(Props, PartialEq)]
struct CalculatorDisplayProps<'a> {
    val: &'a str,
}

fn CalculatorDisplay<'a>(cx: Context<'a, CalculatorDisplayProps>) -> VNode<'a> {
    use separator::Separatable;
    // Todo, add float support to the num-format crate
    let formatted = cx.val.parse::<f64>().unwrap().separated_string();
    // TODO: make it autoscaling with css
    cx.render(rsx! {
        div { class: "calculator-display"
            div { class: "auto-scaling-text", "{formatted}" }
        }
    })
}
