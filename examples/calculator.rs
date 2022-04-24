/*
This example is a simple iOS-style calculator. This particular example can run any platform - Web, Mobile, Desktop.
This calculator version uses React-style state management. All state is held as individual use_states.
*/

use dioxus::events::*;
use dioxus::prelude::*;

fn main() {
    use dioxus::desktop::tao::dpi::LogicalSize;
    dioxus::desktop::launch_cfg(app, |cfg| {
        cfg.with_window(|w| {
            w.with_title("Calculator Demo")
                .with_resizable(false)
                .with_inner_size(LogicalSize::new(320.0, 530.0))
        })
    });
}

fn app(cx: Scope) -> Element {
    let val = use_state(&cx, || String::from("0"));

    let input_digit = move |num: u8| {
        if val.get() == "0" {
            val.set(String::new());
        }
        val.make_mut().push_str(num.to_string().as_str());
    };

    let input_operator = move |key: &str| val.make_mut().push_str(key);

    cx.render(rsx!(
        style { [include_str!("./assets/calculator.css")] }
        div { id: "wrapper",
            div { class: "app",
                div { class: "calculator",
                    onkeydown: move |evt| match evt.key_code {
                        KeyCode::Add => input_operator("+"),
                        KeyCode::Subtract => input_operator("-"),
                        KeyCode::Divide => input_operator("/"),
                        KeyCode::Multiply => input_operator("*"),
                        KeyCode::Num0 => input_digit(0),
                        KeyCode::Num1 => input_digit(1),
                        KeyCode::Num2 => input_digit(2),
                        KeyCode::Num3 => input_digit(3),
                        KeyCode::Num4 => input_digit(4),
                        KeyCode::Num5 => input_digit(5),
                        KeyCode::Num6 => input_digit(6),
                        KeyCode::Num7 => input_digit(7),
                        KeyCode::Num8 => input_digit(8),
                        KeyCode::Num9 => input_digit(9),
                        KeyCode::Backspace => {
                            if !val.len() != 0 {
                                val.make_mut().pop();
                            }
                        }
                        _ => {}
                    },
                    div { class: "calculator-display", [val.to_string()] }
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
                                    [if val.is_empty() { "C" } else { "AC" }]
                                }
                                button {
                                    class: "calculator-key key-sign",
                                    onclick: move |_| {
                                        let temp = calc_val(val.as_str());
                                        if temp > 0.0 {
                                            val.set(format!("-{}", temp));
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
                                button { class: "calculator-key key-0", onclick: move |_| input_digit(0),
                                    "0"
                                }
                                button { class: "calculator-key key-dot", onclick: move |_| val.make_mut().push('.'),
                                    "●"
                                }
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
                            button { class: "calculator-key key-divide", onclick: move |_| input_operator("/"),
                                "÷"
                            }
                            button { class: "calculator-key key-multiply", onclick: move |_| input_operator("*"),
                                "×"
                            }
                            button { class: "calculator-key key-subtract", onclick: move |_| input_operator("-"),
                                "−"
                            }
                            button { class: "calculator-key key-add", onclick: move |_| input_operator("+"),
                                "+"
                            }
                            button { class: "calculator-key key-equals",
                                onclick: move |_| {
                                    val.set(format!("{}", calc_val(val.as_str())));
                                },
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
