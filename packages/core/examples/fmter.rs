// #[macro_use]

// use dioxus_core::ifmt;
// use fstrings::format_args_f;

fn main() {
    let bump = bumpalo::Bump::new();
    let _b = &bump;
    let _world = "123";
    // dioxus_core::ifmt!(in b; "Hello {world}";);
}

// let mut s = bumpalo::collections::String::new_in(b);
// fstrings::write_f!(s, "Hello {world}");
// dbg!(s);
// let p = {
//     println!("hello, {}", &world);
//     ()
// };
// let g = format_args!("hello {world}", world = world);
// let g = dioxus_core::ifmt!(in b, "Hello {world}");
// let g = ifmt!(in b, "hhello {world}");
// let g = ::core::fmt::Arguments::new_v1(
//     &["hello "],
//     &match (&world,) {
//         (arg0,) => [::core::fmt::ArgumentV1::new(
//             arg0,
//             ::core::fmt::Display::fmt,
//         )],
//     },
// );
// fn main() {
//     let bump = bumpalo::Bump::new();
//     let b = &bump;
//     let world = "123";
//     let world = 123;
//     let g = {
//         use bumpalo::core_alloc::fmt::Write;
//         use ::dioxus_core::prelude::bumpalo;
//         use ::dioxus_core::prelude::format_args_f;
//         let bump = b;
//         let mut s = bumpalo::collections::String::new_in(bump);
//         let _ = (&mut s).write_fmt(::core::fmt::Arguments::new_v1(
//             &[""],
//             &match (&::core::fmt::Arguments::new_v1(
//                 &["hhello "],
//                 &match (&world,) {
//                     (arg0,) => [::core::fmt::ArgumentV1::new(
//                         arg0,
//                         ::core::fmt::Display::fmt,
//                     )],
//                 },
//             ),)
//             {
//                 (arg0,) => [::core::fmt::ArgumentV1::new(
//                     arg0,
//                     ::core::fmt::Display::fmt,
//                 )],
//             },
//         ));
//         s
//     };
// }
