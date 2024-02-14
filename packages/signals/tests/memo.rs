// TODO: fix #1935

// #![allow(unused, non_upper_case_globals, non_snake_case)]
// use dioxus_core::NoOpMutations;
// use std::collections::HashMap;
// use std::rc::Rc;

// use dioxus::html::p;
// use dioxus::prelude::*;
// use dioxus_core::ElementId;
// use dioxus_signals::*;
// use std::cell::RefCell;

// #[test]
// fn memos_rerun() {
//     let _ = simple_logger::SimpleLogger::new().init();

//     #[derive(Default)]
//     struct RunCounter {
//         component: usize,
//         effect: usize,
//     }

//     let counter = Rc::new(RefCell::new(RunCounter::default()));
//     let mut dom = VirtualDom::new_with_props(
//         |counter: Rc<RefCell<RunCounter>>| {
//             counter.borrow_mut().component += 1;

//             let mut signal = use_signal(|| 0);
//             let memo = use_memo({
//                 to_owned![counter];
//                 move || {
//                     counter.borrow_mut().effect += 1;
//                     println!("Signal: {:?}", signal);
//                     signal()
//                 }
//             });
//             assert_eq!(memo(), 0);
//             signal += 1;
//             assert_eq!(memo(), 1);

//             rsx! {
//                 div {}
//             }
//         },
//         counter.clone(),
//     );

//     dom.rebuild_in_place();

//     let current_counter = counter.borrow();
//     assert_eq!(current_counter.component, 1);
//     assert_eq!(current_counter.effect, 2);
// }

// #[test]
// fn memos_prevents_component_rerun() {
//     let _ = simple_logger::SimpleLogger::new().init();

//     #[derive(Default)]
//     struct RunCounter {
//         component: usize,
//         memo: usize,
//     }

//     let counter = Rc::new(RefCell::new(RunCounter::default()));
//     let mut dom = VirtualDom::new_with_props(
//         |props: Rc<RefCell<RunCounter>>| {
//             let mut signal = use_signal(|| 0);

//             if generation() == 1 {
//                 *signal.write() = 0;
//             }
//             if generation() == 2 {
//                 println!("Writing to signal");
//                 *signal.write() = 1;
//             }

//             rsx! {
//                 Child {
//                     signal: signal,
//                     counter: props.clone(),
//                 }
//             }
//         },
//         counter.clone(),
//     );

//     #[derive(Default, Props, Clone)]
//     struct ChildProps {
//         signal: Signal<usize>,
//         counter: Rc<RefCell<RunCounter>>,
//     }

//     impl PartialEq for ChildProps {
//         fn eq(&self, other: &Self) -> bool {
//             self.signal == other.signal
//         }
//     }

//     fn Child(props: ChildProps) -> Element {
//         let counter = &props.counter;
//         let signal = props.signal;
//         counter.borrow_mut().component += 1;

//         let memo = use_memo({
//             to_owned![counter];
//             move || {
//                 counter.borrow_mut().memo += 1;
//                 println!("Signal: {:?}", signal);
//                 signal()
//             }
//         });
//         match generation() {
//             0 => {
//                 assert_eq!(memo(), 0);
//             }
//             1 => {
//                 assert_eq!(memo(), 1);
//             }
//             _ => panic!("Unexpected generation"),
//         }

//         rsx! {
//             div {}
//         }
//     }

//     dom.rebuild_in_place();
//     dom.mark_dirty(ScopeId::ROOT);
//     dom.render_immediate(&mut NoOpMutations);

//     {
//         let current_counter = counter.borrow();
//         assert_eq!(current_counter.component, 1);
//         assert_eq!(current_counter.memo, 2);
//     }

//     dom.mark_dirty(ScopeId::ROOT);
//     dom.render_immediate(&mut NoOpMutations);
//     dom.render_immediate(&mut NoOpMutations);

//     {
//         let current_counter = counter.borrow();
//         assert_eq!(current_counter.component, 2);
//         assert_eq!(current_counter.memo, 3);
//     }
// }
