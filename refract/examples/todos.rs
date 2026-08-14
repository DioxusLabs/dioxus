//! A todo list showing path-granular updates: editing one todo reruns only
//! that row's bindings, while adding a todo rebuilds only the list shape.

use refract::prelude::*;

#[derive(PartialEq, Clone, Debug)]
struct Todo {
    title: String,
    done: bool,
}

fn main() {
    let todos = Store::new(vec![
        Todo {
            title: "learn lenses".into(),
            done: true,
        },
        Todo {
            title: "ship refract".into(),
            done: false,
        },
    ]);

    // Equality-gated shape signal: item edits don't rebuild the list.
    let len = memo(move || todos.read().len());
    let remaining = memo(move || todos.read().iter().filter(|t| !t.done).count());

    let view = el("div")
        .child(el("h1").child(dyn_text(move || {
            format!("todos ({} left)", remaining.read())
        })))
        .child(el("ul").dyn_children(move || {
            (0..*len.read())
                .map(|i| {
                    let todo = todos.index::<Todo>(i);
                    let title = lens!(todo => 0: title);
                    let done = lens!(todo => 1: done);
                    el("li")
                        .dyn_attr("class", move || {
                            if *done.read() { "done" } else { "open" }.to_string()
                        })
                        .child(dyn_text(move || title.read().clone()))
                })
                .collect()
        }));

    println!("{}", view.render_to_string());

    // Toggle one todo: only that row's class binding and the counter rerun.
    let done = todos
        .index::<Todo>(1)
        .select(1, |t| &t.done, |t| &mut t.done);
    *done.write() = true;
    println!("{}", view.render_to_string());

    // Add a todo: the list rebuilds, old row bindings are torn down.
    todos.with_mut(|v| {
        v.push(Todo {
            title: "profile it".into(),
            done: false,
        })
    });
    println!("{}", view.render_to_string());
}
