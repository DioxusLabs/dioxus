use dioxus::{core::NoOpMutations, prelude::*};
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, Default)]
struct Captured(Rc<RefCell<Vec<Store<i32>>>>);

#[test]
fn captured_store_lenses_survive_parent_renders() {
    fn app() -> Element {
        let values = use_store(|| vec![10, 20]);
        let index = use_hook(|| Rc::new(std::cell::Cell::new(0)));
        provide_context(index.clone());
        rsx! { Child { value: values.get(index.get()).unwrap(), render: index.get() } }
    }

    #[component]
    fn Child(value: Store<i32>, render: usize) -> Element {
        consume_context::<Captured>().0.borrow_mut().push(value);
        rsx! { "{render}" }
    }

    let captured = Captured::default();
    let mut dom = VirtualDom::new(app);
    dom.insert_any_root_context(Box::new(captured.clone()));
    dom.rebuild_in_place();
    for index in [0, 0, 1, 0] {
        dom.in_scope(ScopeId::APP, || {
            consume_context::<Rc<std::cell::Cell<usize>>>().set(index);
        });
        dom.mark_dirty(ScopeId::APP);
        dom.render_immediate(&mut NoOpMutations);
        for value in captured.0.borrow().iter() {
            assert_eq!(*value.read(), if index == 0 { 10 } else { 20 });
        }
    }
}

#[test]
fn store_props_release_previous_conversions() {
    #[derive(Clone, Default)]
    struct Conversions(Rc<RefCell<Vec<std::rc::Weak<()>>>>);

    fn app() -> Element {
        let values = use_store(|| vec![10]);
        let conversions = consume_context::<Conversions>();
        let token = Rc::new(());
        conversions.0.borrow_mut().push(Rc::downgrade(&token));
        let value: Store<i32, _> = (*values.selector())
            .map(
                move |values: &Vec<i32>| {
                    let _ = &token;
                    &values[0]
                },
                |values: &mut Vec<i32>| &mut values[0],
            )
            .into();
        rsx! { Child { value } }
    }

    #[component]
    fn Child(value: Store<i32>) -> Element {
        let captured = use_hook(|| value);
        assert_eq!(*captured.read(), 10);
        VNode::empty()
    }

    let conversions = Conversions::default();
    let mut dom = VirtualDom::new(app);
    dom.insert_any_root_context(Box::new(conversions.clone()));
    dom.rebuild_in_place();
    for _ in 0..10 {
        dom.mark_dirty(ScopeId::APP);
        dom.render_immediate(&mut NoOpMutations);
    }
    let live = || {
        conversions
            .0
            .borrow()
            .iter()
            .filter(|weak| weak.strong_count() > 0)
            .count()
    };
    assert_eq!(live(), 1);
    drop(dom);
    assert_eq!(live(), 0);
}

#[test]
fn store_aliases_and_read_signals_keep_captured_values_alive() {
    type Aliases = (ReadStore<i32>, WriteStore<i32>, ReadSignal<i32>);
    #[derive(Clone, Default)]
    struct CapturedAliases(Rc<RefCell<Vec<Aliases>>>);

    fn app() -> Element {
        let values = use_store(|| vec![10]);
        rsx! {
            Child {
                read: values.get(0).unwrap(),
                write: values.get(0).unwrap(),
                signal: 10,
            }
        }
    }

    #[component]
    fn Child(read: ReadStore<i32>, write: WriteStore<i32>, signal: ReadSignal<i32>) -> Element {
        consume_context::<CapturedAliases>()
            .0
            .borrow_mut()
            .push((read, write, signal));
        VNode::empty()
    }

    let captured = CapturedAliases::default();
    let mut dom = VirtualDom::new(app);
    dom.insert_any_root_context(Box::new(captured.clone()));
    dom.rebuild_in_place();
    for _ in 0..5 {
        dom.mark_dirty(ScopeId::APP);
        dom.render_immediate(&mut NoOpMutations);
        for (read, write, signal) in captured.0.borrow().iter() {
            assert_eq!((*read.read(), *write.read(), *signal.read()), (10, 10, 10));
        }
    }
}

#[test]
fn unboxed_store_props_still_memoize() {
    fn app() -> Element {
        let value = use_hook(|| Store::<i32, CopyValue<i32>>::new_maybe_sync(10));
        rsx! { Child { value } }
    }

    #[component]
    fn Child(value: Store<i32, CopyValue<i32>>) -> Element {
        assert_eq!(*value.read(), 10);
        VNode::empty()
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
    dom.mark_dirty(ScopeId::APP);
    dom.render_immediate(&mut NoOpMutations);
}
