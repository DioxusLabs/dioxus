//! Export generated binary mutations for the server-free browser tests.
use dioxus_interpreter_js::{NATIVE_JS, SLEDGEHAMMER_JS, unified_bindings::Interpreter};
use std::{fs, path::PathBuf};

fn element(channel: &mut Interpreter, tag: &'static str, id: u32) {
    channel.create_element_top(tag, "");
    channel.set_id(id);
}

fn fixture(name: &'static str, build: impl FnOnce(&mut Interpreter)) -> (&'static str, Vec<u8>) {
    let mut channel = Interpreter::default();
    build(&mut channel);
    (name, channel.export_memory().collect())
}

fn main() {
    let output = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .expect("fixture output directory"),
    );
    fs::create_dir_all(&output).unwrap();
    fs::write(output.join("raw-interpreter.js"), SLEDGEHAMMER_JS).unwrap();
    fs::write(output.join("native-interpreter.js"), NATIVE_JS).unwrap();
    let fixtures = [
        fixture("self", |c| {
            element(c, "button", 1);
            c.foreign_top_event_listener("focus", 0);
            c.foreign_top_event_listener("resize", 0);
            c.append_children_top(1);
            c.push_id(1);
            c.push_id(1);
            c.replace_top_with(1);
            c.push_id(1);
            c.push_id(1);
            c.create_text("sibling");
            c.set_id(2);
            c.replace_top_with(2);
        }),
        fixture("path_self", |c| {
            element(c, "div", 1);
            c.create_text("");
            c.set_id(2);
            c.append_children_top(1);
            c.append_children_top(1);
            c.push_id(1);
            c.child(0);
            c.set_id(2);
            c.push_id(2);
            c.replace_top_with(1);
        }),
        fixture("move_descendant", |c| {
            element(c, "div", 1);
            element(c, "button", 2);
            c.foreign_top_event_listener("focus", 0);
            c.foreign_top_event_listener("resize", 0);
            c.create_text("survivor");
            c.set_id(3);
            c.append_children_top(1);
            c.append_children_top(1);
            c.append_children_top(1);
            c.push_id(1);
            c.push_id(2);
            c.replace_top_with(1);
        }),
        fixture("aliases_and_observers", |c| {
            element(c, "div", 1);
            element(c, "button", 2);
            c.set_id(3);
            c.foreign_top_event_listener("click", 1);
            c.foreign_top_event_listener("click", 1);
            c.foreign_top_event_listener("focus", 0);
            c.foreign_top_event_listener("resize", 0);
            c.foreign_top_event_listener("visible", 0);
            c.append_children_top(1);
            c.append_children_top(1);
            c.push_id(1);
            c.remove_top();
        }),
        fixture("reused_slot", |c| {
            element(c, "div", 1);
            element(c, "button", 2);
            c.foreign_top_event_listener("click", 1);
            c.foreign_top_event_listener("focus", 0);
            c.foreign_top_event_listener("resize", 0);
            c.append_children_top(1);
            c.append_children_top(1);
            element(c, "button", 2);
            c.foreign_top_event_listener("click", 1);
            c.foreign_top_event_listener("focus", 0);
            c.foreign_top_event_listener("resize", 0);
            c.append_children_top(1);
            c.push_id(1);
            c.remove_top();
        }),
        fixture("listener_removal", |c| {
            element(c, "button", 1);
            c.foreign_top_event_listener("focus", 0);
            c.foreign_top_event_listener("focus", 0);
            c.foreign_top_event_listener("click", 1);
            c.foreign_top_event_listener("resize", 0);
            c.remove_top_event_listener("focus", 0);
            c.remove_top_event_listener("resize", 0);
            c.append_children_top(1);
        }),
        fixture("mounted_reuse", |c| {
            element(c, "button", 1);
            c.append_children_top(1);
            c.push_id(1);
            c.queue_top_mounted_event();
            c.remove_top();
            element(c, "button", 1);
            c.queue_top_mounted_event();
            c.append_children_top(1);
        }),
        fixture("prototype_clone", |c| {
            element(c, "section", 10);
            c.create_text("template");
            c.append_children_top(1);
            c.pop();
            c.push_id(10);
            c.clone_node();
            c.set_id(11);
            c.append_children_top(1);
            c.push_id(11);
            c.remove_top();
            c.push_id(10);
            c.clone_node();
            c.set_id(12);
        }),
        fixture("mounted_set_id", |c| {
            element(c, "button", 1);
            c.append_children_top(1);
            c.push_id(1);
            c.set_id(2);
            c.queue_top_mounted_event();
            c.pop();
        }),
    ];
    let entries = fixtures
        .iter()
        .map(|(name, bytes)| format!("\"{name}\":{bytes:?}"))
        .collect::<Vec<_>>()
        .join(",");
    fs::write(output.join("fixtures.json"), format!("{{{entries}}}")).unwrap();
}
