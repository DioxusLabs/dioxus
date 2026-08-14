//! Todos: index lenses, memoized derived values, and a dynamic list.

use refract::{Lens, Ui, VecLens, dyn_text, el, lens};

#[derive(PartialEq, Clone)]
struct Todo {
    title: String,
    done: bool,
}

struct App {
    todos: Vec<Todo>,
}

fn main() {
    let mut ui = Ui::new(App {
        todos: vec![
            Todo {
                title: "learn lenses".into(),
                done: true,
            },
            Todo {
                title: "build refract".into(),
                done: false,
            },
        ],
    });
    let todos = lens!(App => 0: todos);

    // Memos cut off structural over-invalidation: editing one todo's fields
    // dirties these, but they only propagate if the value actually changed.
    let len = ui.memo(move |ctx| ctx.get(todos).len());
    let remaining = ui.memo(move |ctx| ctx.get(todos).iter().filter(|t| !t.done).count());

    let root = ui.mount(
        el("div")
            .child(el("p").child(dyn_text(move |ctx| {
                // `read_memo` borrows the ctx mutably, so copy out
                // one value before reading the next.
                let left = *ctx.read_memo(remaining);
                let total = *ctx.read_memo(len);
                format!("{left} left of {total}")
            })))
            .child(el("ul").dyn_children(move |ctx| {
                let n = *ctx.read_memo(len);
                (0..n)
                    .map(|i| {
                        let title = todos.at(i).field(0, |t: &Todo| &t.title, |t| &mut t.title);
                        let done = todos.at(i).field(1, |t: &Todo| &t.done, |t| &mut t.done);
                        el("li")
                            .dyn_attr("class", move |ctx| {
                                if *ctx.get(done) { "done" } else { "open" }.to_string()
                            })
                            .child(dyn_text(move |ctx| ctx.get(title).clone()))
                            .into()
                    })
                    .collect()
            })),
    );

    println!("{}", ui.render_to_string(root));

    // Toggle one todo: only that item's bindings update.
    let second_done = todos.at(1).field(1, |t: &Todo| &t.done, |t| &mut t.done);
    ui.with(|ctx| *ctx.write(second_done) = true);
    println!("{}", ui.render_to_string(root));

    // Push a new todo: the list rebuilds.
    ui.with(|ctx| {
        ctx.write(todos).push(Todo {
            title: "ship it".into(),
            done: false,
        })
    });
    println!("{}", ui.render_to_string(root));
}
