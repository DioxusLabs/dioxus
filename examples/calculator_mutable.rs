//! This example showcases a simple calculator using an approach to state management where the state is composed of only
//! a single signal. Since Dioxus implements traditional React diffing, state can be consolidated into a typical Rust struct
//! with methods that take `&mut self`. For many use cases, this is a simple way to manage complex state without wrapping
//! everything in a signal.
//!
//! Generally, you'll want to split your state into several signals if you have a large application, but for small
//! applications, or focused components, this is a great way to manage state.

use dioxus::desktop::tao::dpi::LogicalSize;
use dioxus::desktop::{Config, WindowBuilder};
use dioxus::html::input_data::keyboard_types::Key;
use dioxus::html::MouseEvent;
use dioxus::prelude::*;

fn main() {
    LaunchBuilder::desktop()
        .with_cfg(
            Config::new().with_window(
                WindowBuilder::new()
                    .with_title("Calculator Demo")
                    .with_resizable(false)
                    .with_inner_size(LogicalSize::new(320.0, 530.0)),
            ),
        )
        .launch(app);
}

const STYLE: &str = include_str!("./assets/calculator.css");

fn app() -> Element {
    let mut state = use_signal(Calculator::new);

    rsx! {
        style { {STYLE} }
        div { id: "wrapper",
            div { class: "app",
                div {
                    class: "calculator",
                    onkeypress: move |evt| state.write().handle_keydown(evt),
                    div { class: "calculator-display", {state.read().formatted_display()} }
                    div { class: "calculator-keypad",
                        div { class: "input-keys",
                            div { class: "function-keys",
                                CalculatorKey { name: "key-clear", onclick: move |_| state.write().clear_display(),
                                    if state.read().display_value == "0" { "C" } else { "AC" }
                                }
                                CalculatorKey { name: "key-sign", onclick: move |_| state.write().toggle_sign(), "±" }
                                CalculatorKey { name: "key-percent", onclick: move |_| state.write().toggle_percent(), "%" }
                            }
                            div { class: "digit-keys",
                                CalculatorKey { name: "key-0", onclick: move |_| state.write().input_digit(0), "0" }
                                CalculatorKey { name: "key-dot", onclick: move |_| state.write().input_dot(), "●" }
                                for k in 1..10 {
                                    CalculatorKey {
                                        key: "{k}",
                                        name: "key-{k}",
                                        onclick: move |_| state.write().input_digit(k),
                                        "{k}"
                                    }
                                }
                            }
                        }
                        div { class: "operator-keys",
                            CalculatorKey {
                                name: "key-divide",
                                onclick: move |_| state.write().set_operator(Operator::Div),
                                "÷"
                            }
                            CalculatorKey {
                                name: "key-multiply",
                                onclick: move |_| state.write().set_operator(Operator::Mul),
                                "×"
                            }
                            CalculatorKey {
                                name: "key-subtract",
                                onclick: move |_| state.write().set_operator(Operator::Sub),
                                "−"
                            }
                            CalculatorKey { name: "key-add", onclick: move |_| state.write().set_operator(Operator::Add), "+" }
                            CalculatorKey { name: "key-equals", onclick: move |_| state.write().perform_operation(), "=" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn CalculatorKey(name: String, onclick: EventHandler<MouseEvent>, children: Element) -> Element {
    rsx! {
        button { class: "calculator-key {name}", onclick: move |e| onclick.call(e), {&children} }
    }
}

struct Calculator {
    display_value: String,
    operator: Option<Operator>,
    waiting_for_operand: bool,
    cur_val: f64,
}

#[derive(Clone)]
enum Operator {
    Add,
    Sub,
    Mul,
    Div,
}

impl Calculator {
    fn new() -> Self {
        Calculator {
            display_value: "0".to_string(),
            operator: None,
            waiting_for_operand: false,
            cur_val: 0.0,
        }
    }
    fn formatted_display(&self) -> String {
        use separator::Separatable;
        self.display_value
            .parse::<f64>()
            .unwrap()
            .separated_string()
    }
    fn clear_display(&mut self) {
        self.display_value = "0".to_string();
    }
    fn input_digit(&mut self, digit: u8) {
        let content = digit.to_string();
        if self.waiting_for_operand || self.display_value == "0" {
            self.waiting_for_operand = false;
            self.display_value = content;
        } else {
            self.display_value.push_str(content.as_str());
        }
    }
    fn input_dot(&mut self) {
        if !self.display_value.contains('.') {
            self.display_value.push('.');
        }
    }
    fn perform_operation(&mut self) {
        if let Some(op) = &self.operator {
            let rhs = self.display_value.parse::<f64>().unwrap();
            let new_val = match op {
                Operator::Add => self.cur_val + rhs,
                Operator::Sub => self.cur_val - rhs,
                Operator::Mul => self.cur_val * rhs,
                Operator::Div => self.cur_val / rhs,
            };
            self.cur_val = new_val;
            self.display_value = new_val.to_string();
            self.operator = None;
        }
    }
    fn toggle_sign(&mut self) {
        if self.display_value.starts_with('-') {
            self.display_value = self.display_value.trim_start_matches('-').to_string();
        } else {
            self.display_value = format!("-{}", self.display_value);
        }
    }
    fn toggle_percent(&mut self) {
        self.display_value = (self.display_value.parse::<f64>().unwrap() / 100.0).to_string();
    }
    fn backspace(&mut self) {
        if !self.display_value.as_str().eq("0") {
            self.display_value.pop();
        }
    }
    fn set_operator(&mut self, operator: Operator) {
        self.operator = Some(operator);
        self.cur_val = self.display_value.parse::<f64>().unwrap();
        self.waiting_for_operand = true;
    }
    fn handle_keydown(&mut self, evt: KeyboardEvent) {
        match evt.key() {
            Key::Backspace => self.backspace(),
            Key::Character(c) => match c.as_str() {
                "0" => self.input_digit(0),
                "1" => self.input_digit(1),
                "2" => self.input_digit(2),
                "3" => self.input_digit(3),
                "4" => self.input_digit(4),
                "5" => self.input_digit(5),
                "6" => self.input_digit(6),
                "7" => self.input_digit(7),
                "8" => self.input_digit(8),
                "9" => self.input_digit(9),
                "+" => self.operator = Some(Operator::Add),
                "-" => self.operator = Some(Operator::Sub),
                "/" => self.operator = Some(Operator::Div),
                "*" => self.operator = Some(Operator::Mul),
                _ => {}
            },

            _ => {}
        }
    }
}
