//! Example: Calculator
//! -------------------
//!
//! Some components benefit through the use of "Models". Models are a single block of encapsulated state that allow mutative
//! methods to be performed on them. Dioxus exposes the ability to use the model pattern through the "use_model" hook.
//!
//! Models are commonly used in the "Model-View-Component" approach for building UI state.
//!
//! `use_model` is basically just a fancy wrapper around set_state, but saves a "working copy" of the new state behind a
//! RefCell. To modify the working copy, you need to call "get_mut" which returns the RefMut. This makes it easy to write
//! fully encapsulated apps that retain a certain feel of native Rusty-ness. A calculator app is a good example of when this
//! is useful.
//!
//! Do note that "get_mut" returns a `RefMut` (a lock over a RefCell). If two `RefMut`s are held at the same time (ie in a loop)
//! the RefCell will panic and crash. You can use `try_get_mut` or `.modify` to avoid this problem, or just not hold two
//! RefMuts at the same time.

use dioxus::events::on::*;
use dioxus::prelude::*;

fn main() {
    dioxus::webview::launch(App)
}

static App: FC<()> = |cx| {
    let calc = use_model(&cx, || Calculator::new());

    let clear_display = calc.display_value.eq("0");
    let clear_text = if clear_display { "C" } else { "AC" };
    let formatted = calc.formatted();

    cx.render(rsx! {
        div { class: "calculator", onkeydown: move |evt| calc.get_mut().handle_keydown(evt),
            div { class: "calculator-display", "{formatted}" }
            div { class: "input-keys"
                div { class: "function-keys"
                    CalculatorKey { name: "key-clear", onclick: move |_| calc.get_mut().clear_display(), "{clear_text}" }
                    CalculatorKey { name: "key-sign", onclick: move |_| calc.get_mut().toggle_sign(), "±"}
                    CalculatorKey { name: "key-percent", onclick: move |_| calc.get_mut().toggle_percent(), "%"}
                }
                div { class: "digit-keys"
                    CalculatorKey { name: "key-0", onclick: move |_| calc.get_mut().input_digit(0), "0" }
                    CalculatorKey { name: "key-dot", onclick: move |_|  calc.get_mut().input_dot(), "●" }
                    {(1..9).map(move |k| rsx!{
                        CalculatorKey { key: "{k}", name: "key-{k}", onclick: move |_|  calc.get_mut().input_digit(k), "{k}" }
                    })}
                }
                div { class: "operator-keys"
                    CalculatorKey { name:"key-divide", onclick: move |_| calc.get_mut().set_operator(Operator::Div) "÷" }
                    CalculatorKey { name:"key-multiply", onclick: move |_| calc.get_mut().set_operator(Operator::Mul) "×" }
                    CalculatorKey { name:"key-subtract", onclick: move |_| calc.get_mut().set_operator(Operator::Sub) "−" }
                    CalculatorKey { name:"key-add", onclick: move |_| calc.get_mut().set_operator(Operator::Add) "+" }
                    CalculatorKey { name:"key-equals", onclick: move |_| calc.get_mut().perform_operation() "=" }
                }
            }
        }
    })
};

#[derive(Props)]
struct CalculatorKeyProps<'a> {
    name: &'static str,
    onclick: &'a dyn Fn(MouseEvent),
}

fn CalculatorKey<'a>(cx: Context<'a, CalculatorKeyProps>) -> VNode<'a> {
    cx.render(rsx! {
        button {
            class: "calculator-key {cx.name}"
            onclick: {cx.onclick}
            {cx.children()}
        }
    })
}

#[derive(Clone)]
struct Calculator {
    display_value: String,
    operator: Option<Operator>,
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
            cur_val: 0.0,
        }
    }
    fn formatted(&self) -> String {
        use separator::Separatable;
        self.cur_val.separated_string()
    }
    fn clear_display(&mut self) {
        self.display_value = "0".to_string();
    }
    fn input_digit(&mut self, digit: u8) {
        self.display_value.push_str(digit.to_string().as_str())
    }
    fn input_dot(&mut self) {
        if self.display_value.find(".").is_none() {
            self.display_value.push_str(".");
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
        if self.display_value.starts_with("-") {
            self.display_value = self.display_value.trim_start_matches("-").to_string();
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
        self.operator = Some(operator)
    }
    fn handle_keydown(&mut self, evt: KeyboardEvent) {
        match evt.key_code() {
            KeyCode::Backspace => self.backspace(),
            KeyCode::_0 => self.input_digit(0),
            KeyCode::_1 => self.input_digit(1),
            KeyCode::_2 => self.input_digit(2),
            KeyCode::_3 => self.input_digit(3),
            KeyCode::_4 => self.input_digit(4),
            KeyCode::_5 => self.input_digit(5),
            KeyCode::_6 => self.input_digit(6),
            KeyCode::_7 => self.input_digit(7),
            KeyCode::_8 => self.input_digit(8),
            KeyCode::_9 => self.input_digit(9),
            KeyCode::Add => self.operator = Some(Operator::Add),
            KeyCode::Subtract => self.operator = Some(Operator::Sub),
            KeyCode::Divide => self.operator = Some(Operator::Div),
            KeyCode::Multiply => self.operator = Some(Operator::Mul),
            _ => {}
        }
    }
}
