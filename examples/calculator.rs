//! Calculator
//!
//! This example is a simple iOS-style calculator. Instead of wrapping the state in a single struct like the
//! `calculate_mutable` example, this example uses several closures to manage actions with the state. Most
//! components will start like this since it's the quickest way to start adding state to your app. The `Signal` type
//! in Dioxus is `Copy` - meaning you don't need to clone it to use it in a closure.
//!
//! Notice how our logic is consolidated into just a few callbacks instead of a single struct. This is a rather organic
//! way to start building state management in Dioxus, and it's a great way to start.

use dioxus::events::*;
use dioxus::html::input_data::keyboard_types::Key;
use dioxus::prelude::*;

const STYLE: Asset = asset!("/examples/assets/calculator.css");

fn main() {
    LaunchBuilder::desktop()
        .with_cfg(desktop!({
            use dioxus::desktop::{Config, LogicalSize, WindowBuilder};
            Config::new().with_window(
                WindowBuilder::default()
                    .with_title("Calculator")
                    .with_inner_size(LogicalSize::new(300.0, 525.0)),
            )
        }))
        .launch(app);
}

fn app() -> Element {
    let mut val = use_signal(|| String::from("0"));

    let mut input_digit = move |num: String| {
        if val() == "0" {
            val.set(String::new());
        }
        val.write().push_str(num.as_str());
    };

    let mut input_operator = move |key: &str| val.write().push_str(key);

    let handle_key_down_event = move |evt: KeyboardEvent| match evt.key() {
        Key::Backspace => {
            if !val().is_empty() {
                val.write().pop();
            }
        }
        Key::Character(character) => match character.as_str() {
            "+" | "-" | "/" | "*" => input_operator(&character),
            "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" => input_digit(character),
            _ => {}
        },
        _ => {}
    };

    rsx! {
        document::Stylesheet { href: STYLE }
        div { id: "wrapper",
            div { class: "app",
                div { class: "calculator", tabindex: "0", onkeydown: handle_key_down_event,
                    div { class: "calculator-display",
                        if val().is_empty() {
                            "0"
                        } else {
                            "{val}"
                        }
                    }
                    div { class: "calculator-keypad",
                        div { class: "input-keys",
                            div { class: "function-keys",
                                button {
                                    class: "calculator-key key-clear",
                                    onclick: move |_| {
                                        val.set(String::new());
                                        if !val.cloned().is_empty() {
                                            val.set("0".into());
                                        }
                                    },
                                    if val.cloned().is_empty() { "C" } else { "AC" }
                                }
                                button {
                                    class: "calculator-key key-sign",
                                    onclick: move |_| {
                                        let new_val = calc_val(val.cloned().as_str());
                                        if new_val > 0.0 {
                                            val.set(format!("-{new_val}"));
                                        } else {
                                            val.set(format!("{}", new_val.abs()));
                                        }
                                    },
                                    "±"
                                }
                                button {
                                    class: "calculator-key key-percent",
                                    onclick: move |_| val.set(format!("{}", calc_val(val.cloned().as_str()) / 100.0)),
                                    "%"
                                }
                            }
                            div { class: "digit-keys",
                                button {
                                    class: "calculator-key key-0",
                                    onclick: move |_| input_digit(0.to_string()),
                                    "0"
                                }
                                button {
                                    class: "calculator-key key-dot",
                                    onclick: move |_| val.write().push('.'),
                                    "●"
                                }
                                for k in 1..10 {
                                    button {
                                        class: "calculator-key {k}",
                                        name: "key-{k}",
                                        onclick: move |_| input_digit(k.to_string()),
                                        "{k}"
                                    }
                                }
                            }
                        }
                        div { class: "operator-keys",
                            for (key, class) in [("/", "key-divide"), ("*", "key-multiply"), ("-", "key-subtract"), ("+", "key-add")] {
                                button {
                                    class: "calculator-key {class}",
                                    onclick: move |_| input_operator(key),
                                    "{key}"
                                }
                            }
                            button {
                                class: "calculator-key key-equals",
                                onclick: move |_| val.set(format!("{}", calc_val(val.cloned().as_str()))),
                                "="
                            }
                        }
                    }
                }
            }
        }
    }
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
