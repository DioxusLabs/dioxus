//! Tests the use_drop hook
use dioxus::dioxus_core::use_drop;
use dioxus::prelude::*;
use std::sync::{Arc, Mutex};

type Shared<T> = Arc<Mutex<T>>;

#[derive(Clone, Props)]
struct AppProps {
    render_child: Shared<bool>,
    drop_count: Shared<u32>,
}

impl PartialEq for AppProps {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.drop_count, &other.drop_count)
    }
}

fn app(props: AppProps) -> Element {
    let render_child = props.render_child.clone();
    let render_child = *render_child.lock().unwrap();
    println!(
        "Rendering app component with render_child: {}",
        render_child
    );
    rsx! {
        if render_child {
            child_component {
                drop_count: props.drop_count.clone(),
                render_child: props.render_child.clone()
            }
        }
    }
}

fn child_component(props: AppProps) -> Element {
    println!("Rendering child component");
    use_drop(move || {
        println!("Child component is being dropped");
        let mut count = props.drop_count.lock().unwrap();
        *count += 1;
    });

    rsx! {}
}

#[test]
fn drop_runs() {
    let drop_count = Arc::new(Mutex::new(0));
    let render_child = Arc::new(Mutex::new(true));
    let mut dom = VirtualDom::new_with_props(
        app,
        AppProps { drop_count: drop_count.clone(), render_child: render_child.clone() },
    );

    dom.rebuild_in_place();

    assert_eq!(*drop_count.lock().unwrap(), 0);
    *render_child.lock().unwrap() = false;

    dom.mark_dirty(ScopeId::APP);
    dom.render_immediate(&mut dioxus_core::NoOpMutations);

    assert_eq!(*drop_count.lock().unwrap(), 1);
    *render_child.lock().unwrap() = false;
}
