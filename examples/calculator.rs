/*
This example is a simple iOS-style calculator. This particular example can run any platform - Web, Mobile, Desktop.
This calculator version uses React-style state management. All state is held as individual use_states.
*/

use dioxus::events::*;
use dioxus::html::input_data::keyboard_types::Key;
use dioxus::prelude::*;
use dioxus_desktop::{Config, WindowBuilder};

fn main() {
    let config = Config::new().with_window(
        WindowBuilder::default()
            .with_title("Calculator")
            .with_inner_size(dioxus_desktop::LogicalSize::new(300.0, 500.0)),
    );

    dioxus_desktop::launch_cfg(app, config);
}

fn app(cx: Scope) -> Element {
    let val = use_state(cx, || String::from("0"));

    let input_digit = move |num: u8| {
        if val.get() == "0" {
            val.set(String::new());
        }

        val.make_mut().push_str(num.to_string().as_str());
    };

    let input_operator = move |key: &str| val.make_mut().push_str(key);

    let handle_key_down_event = move |evt: KeyboardEvent| match evt.key() {
        Key::Backspace => {
            if !val.len() != 0 {
                val.make_mut().pop();
            }
        }
        Key::Character(character) => match character.as_str() {
            "+" => input_operator("+"),
            "-" => input_operator("-"),
            "/" => input_operator("/"),
            "*" => input_operator("*"),
            "0" => input_digit(0),
            "1" => input_digit(1),
            "2" => input_digit(2),
            "3" => input_digit(3),
            "4" => input_digit(4),
            "5" => input_digit(5),
            "6" => input_digit(6),
            "7" => input_digit(7),
            "8" => input_digit(8),
            "9" => input_digit(9),
            _ => {}
        },
        _ => {}
    };

    cx.render(rsx!(
        style { include_str!("./assets/calculator.css") }
        div { id: "wrapper",
            div { class: "app",
                div { class: "calculator",
                    tabindex: "0",
                    onkeydown: handle_key_down_event,
                    div { class: "calculator-display", val.to_string() }
                    div { class: "calculator-keypad",
                        div { class: "input-keys",
                            div { class: "function-keys",
                                button {
                                    class: "calculator-key key-clear",
                                    onclick: move |_| {
                                        val.set(String::new());
                                        if !val.is_empty(){
                                            val.set("0".into());
                                        }
                                    },
                                    if val.is_empty() { "C" } else { "AC" }
                                }
                                button {
                                    class: "calculator-key key-sign",
                                    onclick: move |_| {
                                        let temp = calc_val(val.as_str());
                                        if temp > 0.0 {
                                            val.set(format!("-{temp}"));
                                        } else {
                                            val.set(format!("{}", temp.abs()));
                                        }
                                    },
                                    "±"
                                }
                                button {
                                    class: "calculator-key key-percent",
                                    onclick: move |_| {
                                        val.set(
                                            format!("{}", calc_val(val.as_str()) / 100.0)
                                        );
                                    },
                                    "%"
                                }
                            }
                            div { class: "digit-keys",
                                button { class: "calculator-key key-0", onclick: move |_| input_digit(0), "0" }
                                button { class: "calculator-key key-dot", onclick: move |_| val.make_mut().push('.'), "●" }
                                (1..10).map(|k| rsx!{
                                    button {
                                        class: "calculator-key {k}",
                                        name: "key-{k}",
                                        onclick: move |_| input_digit(k),
                                        "{k}"
                                    }
                                }),
                            }
                        }
                        div { class: "operator-keys",
                            button { class: "calculator-key key-divide", onclick: move |_| input_operator("/"), "÷" }
                            button { class: "calculator-key key-multiply", onclick: move |_| input_operator("*"), "×" }
                            button { class: "calculator-key key-subtract", onclick: move |_| input_operator("-"), "−" }
                            button { class: "calculator-key key-add", onclick: move |_| input_operator("+"), "+" }
                            button {
                                class: "calculator-key key-equals",
                                onclick: move |_| val.set(format!("{}", calc_val(val.as_str()))),
                                "="
                            }
                        }
                    }
                }
            }
        }

    ))
}

fn calc_val(val: &str) -> f64 {
    let mut temp = String::new();
    let mut operation = "+".to_string();

    let mut start_index = 0;
    let mut temp_value;
    let mut fin_index = 0;

    if &val[0..1] == "-" {
        temp_value = String::from("-");
        fin_index = 1;
        start_index += 1;
    } else {
        temp_value = String::from("");
    }

    for c in val[fin_index..].chars() {
        if c == '+' || c == '-' || c == '*' || c == '/' {
            break;
        }
        temp_value.push(c);
        start_index += 1;
    }

    let mut result = temp_value.parse::<f64>().unwrap();

    if start_index + 1 >= val.len() {
        return result;
    }

    for c in val[start_index..].chars() {
        if c == '+' || c == '-' || c == '*' || c == '/' {
            if !temp.is_empty() {
                match &operation as &str {
                    "+" => result += temp.parse::<f64>().unwrap(),
                    "-" => result -= temp.parse::<f64>().unwrap(),
                    "*" => result *= temp.parse::<f64>().unwrap(),
                    "/" => result /= temp.parse::<f64>().unwrap(),
                    _ => unreachable!(),
                };
            }
            operation = c.to_string();
            temp = String::new();
        } else {
            temp.push(c);
        }
    }

    if !temp.is_empty() {
        match &operation as &str {
            "+" => result += temp.parse::<f64>().unwrap(),
            "-" => result -= temp.parse::<f64>().unwrap(),
            "*" => result *= temp.parse::<f64>().unwrap(),
            "/" => result /= temp.parse::<f64>().unwrap(),
            _ => unreachable!(),
        };
    }

    result
}
