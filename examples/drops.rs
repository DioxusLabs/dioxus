use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    let count = if cx.generation() % 2 == 0 { 10 } else { 0 };

    println!("Generation: {}", cx.generation());

    if cx.generation() < 10 {
        cx.needs_update();
    }

    render! {
        for _ in 0..count {
            drop_child {}
        }
    }
}

fn drop_child(cx: Scope) -> Element {
    cx.use_hook(|| Drops);
    render! {
        div{}
    }
}

struct Drops;

impl Drop for Drops {
    fn drop(&mut self) {
        println!("Dropped!");
    }
}
