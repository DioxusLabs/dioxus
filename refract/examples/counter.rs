//! Counter: field lenses, a memo, and dynamic text bindings.

use refract::{Ui, dyn_text, el, lens, text};

struct App {
    count: i32,
    step: i32,
}

fn main() {
    let mut ui = Ui::new(App { count: 0, step: 1 });
    let count = lens!(App => 0: count);
    let step = lens!(App => 1: step);

    let doubled = ui.memo(move |ctx| *ctx.get(count) * 2);

    let root = ui.mount(
        el("div")
            .child(
                el("span")
                    .child(text("count: "))
                    .child(dyn_text(move |ctx| ctx.get(count).to_string())),
            )
            .child(
                el("span")
                    .child(text(" doubled: "))
                    .child(dyn_text(move |ctx| ctx.read_memo(doubled).to_string())),
            ),
    );

    println!("{}", ui.render_to_string(root));

    for _ in 0..3 {
        ui.with(|ctx| {
            let by = *ctx.peek(step);
            *ctx.write(count) += by;
        });
        println!("{}", ui.render_to_string(root));
    }

    // Bump the step, then count once more.
    ui.with(|ctx| *ctx.write(step) = 10);
    ui.with(|ctx| {
        let by = *ctx.peek(step);
        *ctx.write(count) += by;
    });
    println!("{}", ui.render_to_string(root));
}
