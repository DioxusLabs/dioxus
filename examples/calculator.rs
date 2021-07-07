//! Example: Calculator
//! -------------------------

// use dioxus::events::on::*;
// use dioxus::prelude::*;

fn main() {
    dioxus::desktop::launch(App);
}

use dioxus::events::on::*;
use dioxus::prelude::*;

enum Operator {
    Add,
    Sub,
    Mul,
    Div,
}

static App: FC<()> = |cx| {
    let cur_val = use_state(cx, || 0.0_f64);
    let operator = use_state(cx, || None as Option<Operator>);
    let display_value = use_state(cx, || "".to_string());

    let clear_display = display_value.eq("0");
    let clear_text = if clear_display { "C" } else { "AC" };

    let input_digit =
        move |num: u8| display_value.modify(move |f| f.push_str(num.to_string().as_str()));

    let input_dot = move || display_value.modify(move |f| f.push_str("."));

    let perform_operation = move || {
        if let Some(op) = operator.as_ref() {
            let rhs = display_value.parse::<f64>().unwrap();
            let new_val = match op {
                Operator::Add => **cur_val + rhs,
                Operator::Sub => **cur_val - rhs,
                Operator::Mul => **cur_val * rhs,
                Operator::Div => **cur_val / rhs,
            };
            cur_val.set(new_val);
            display_value.set(new_val.to_string());
            operator.set(None);
        }
    };

    let toggle_sign = move |_| {
        if display_value.starts_with("-") {
            display_value.set(display_value.trim_start_matches("-").to_string())
        } else {
            display_value.set(format!("-{}", **display_value))
        }
    };
    let toggle_percent = move |_| todo!();

    let clear_key = move |_| {
        display_value.set("0".to_string());
        if !clear_display {
            operator.set(None);
            cur_val.set(0.0);
        }
    };

    let keydownhandler = move |evt: KeyboardEvent| match evt.key_code() {
        KeyCode::Backspace => display_value.modify(|f| {
            if !f.as_str().eq("0") {
                f.pop();
            }
        }),
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
        KeyCode::Add => operator.set(Some(Operator::Add)),
        KeyCode::Subtract => operator.set(Some(Operator::Sub)),
        KeyCode::Divide => operator.set(Some(Operator::Div)),
        KeyCode::Multiply => operator.set(Some(Operator::Mul)),
        _ => {}
    };

    cx.render(rsx! {
        div { class: "calculator", onkeydown: {keydownhandler}
            div { class: "input-keys"
                div { class: "function-keys"
                    CalculatorKey { name: "key-clear", onclick: {clear_key} "{clear_text}" }
                    CalculatorKey { name: "key-sign", onclick: {toggle_sign}, "±"}
                    CalculatorKey { name: "key-percent", onclick: {toggle_percent} "%"}
                }
                div { class: "digit-keys"
                    CalculatorKey { name: "key-0", onclick: move |_| input_digit(0), "0" }
                    CalculatorKey { name: "key-dot", onclick: move |_|  input_dot(), "●" }

                    {(1..9).map(move |k| rsx!{
                        CalculatorKey { key: "{k}", name: "key-{k}", onclick: move |_|  input_digit(k), "{k}" }
                    })}
                }
                div { class: "operator-keys"
                    CalculatorKey { name: "key-divide", onclick: move |_| operator.set(Some(Operator::Div)) "÷" }
                    CalculatorKey { name: "key-multiply", onclick: move |_| operator.set(Some(Operator::Mul)) "×" }
                    CalculatorKey { name: "key-subtract", onclick: move |_| operator.set(Some(Operator::Sub)) "−" }
                    CalculatorKey { name: "key-add", onclick: move |_| operator.set(Some(Operator::Add)) "+" }
                    CalculatorKey { name: "key-equals", onclick: move |_| perform_operation() "=" }
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
struct CalculatorDisplayProps {
    val: f64,
}

fn CalculatorDisplay(cx: Context<CalculatorDisplayProps>) -> VNode {
    use separator::Separatable;
    // Todo, add float support to the num-format crate
    let formatted = cx.val.separated_string();
    // TODO: make it autoscaling with css
    cx.render(rsx! {
        div { class: "calculator-display"
            "{formatted}"
        }
    })
}
