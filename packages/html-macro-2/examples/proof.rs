use bumpalo::Bump;
use dioxus_html_2::html;

mod dioxus {
    pub use bumpalo;
    pub mod builder {
        use bumpalo::Bump;
    }
}

fn main() {
    /*
    Th below code is not meant to compile, but it is meant to expand properly


    */
    // let l = html! {
    //     <div>
    //         <div>
    //             <h1>"asdl"</h1>
    //             <h1>"asdl"</h1>
    //             <h1>"asdl"</h1>
    //             <h1>"asdl"</h1>
    //         </div>
    //     </div>
    // };

    // let l = move |bump| {
    //     dioxus::builder::div(bump)
    //         .children([dioxus::builder::div(bump)
    //             .children([
    //                 dioxus::builder::h1(bump)
    //                     .children([dioxus::builder::text("asdl")])
    //                     .finish(),
    //                 dioxus::builder::h1(bump)
    //                     .children([dioxus::builder::text("asdl")])
    //                     .finish(),
    //                 dioxus::builder::h1(bump)
    //                     .children([dioxus::builder::text("asdl")])
    //                     .finish(),
    //                 dioxus::builder::h1(bump)
    //                     .children([dioxus::builder::text("asdl")])
    //                     .finish(),
    //             ])
    //             .finish()])
    //         .finish()
    // };
}
