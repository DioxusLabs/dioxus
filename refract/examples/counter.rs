//! The classic counter, driven from the terminal instead of click events.

use refract::prelude::*;

#[derive(PartialEq)]
struct App {
    count: i32,
    step: i32,
}

fn main() {
    let app = Store::new(App { count: 0, step: 1 });
    let count = lens!(app => 0: count);
    let step = lens!(app => 1: step);

    let doubled = memo(move || *count.read() * 2);

    let view = el("div")
        .child(el("span").child(dyn_text(move || format!("count: {}", count.read()))))
        .child(el("span").child(dyn_text(move || format!(" doubled: {}", doubled.read()))));

    println!("{}", view.render_to_string());

    // Simulate three "increment" clicks.
    for _ in 0..3 {
        let by = *step.peek();
        *count.write() += by;
        println!("{}", view.render_to_string());
    }

    // Changing `step` does not re-render anything: nothing subscribes to it.
    step.set(10);
    let by = *step.peek();
    *count.write() += by;
    println!("{}", view.render_to_string());
}
