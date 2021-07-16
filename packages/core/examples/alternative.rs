fn main() {}

// use dioxus::*;
// use dioxus_core as dioxus;
// use dioxus_core::prelude::*;

// pub static Example: FC<()> = |cx| {
//     let list = (0..10).map(|f| LazyNodes::new(move |f| todo!()));

//     cx.render(LazyNodes::new(move |cx| {
//         let bump = cx.bump();
//         cx.raw_element("div")
//             .children([
//                 cx.text(format_args!("hello")),
//                 cx.text(format_args!("hello")),
//                 cx.fragment_from_iter(list),
//             ])
//             .finish()
//     }))
// };
