use dioxus_core::prelude::*;

fn main() {}

fn Example(ctx: Context, props: ()) -> DomTree {
    ctx.render(rsx! {
        div {
            <h1 tag="type" abc=123 class="big..."> "title1" "title2" </h1>

            <h1 alsd> alkjsd </h1>


            h1 { tag: "type", abc: 123, class: "big small wide short",
                "title1"
                "title1"
                "title1"
                "title"
            }

            h1 ("title") {
                 tag: "type",
                 abc: 123,
                 class: "big small wide short",
            }

            // <button
            //     class="inline-block py-4 px-8 mr-6 leading-none text-white bg-indigo-600 hover:bg-indigo-900 font-semibold rounded shadow"
            //     onclick={move |_| set_name("jill")}
            //     onclick={move |_| set_name("jill")}
            // >
            //     "Jill!"
            // </button>
            

            button { "Jill!",
                class: "inline-block py-4 px-8 mr-6 leading-none text-white bg-indigo-600 hover:bg-indigo-900 font-semibold rounded shadow"
                onclick: move |_| set_name("jill"),
                onclick: move |_| set_name("jill"),
            }

            button {
                class: "inline-block py-4 px-8 mr-6 leading-none text-white bg-indigo-600 hover:bg-indigo-900 font-semibold rounded shadow"
                onclick: move |_| set_name("jill"),
                onclick: move |_| set_name("jill"),
                // this is valid
                "Jill!",
                // this is also valid
                {"Jill!"}
            }

            h1 { "Text", class: "inline-block py-4 px-8 mr-6 leading-none" }

            // <h1 class="inline-block py-4 px-8 mr-6 leading-none">
            //     "Text"
            // </h1>

            h1 {
                div {
                    h1 {}
                    h2 {}
                    Brick {}

                    p {}
                    p {
                        tag: "type", 
                        abc: 123, 
                        enabled: true,
                        class: "big small wide short",

                        a { "abcder" },
                        h2 { "whatsup", class: "abc-123" },
                        CustomComponent { a: 123, b: 456, key: "1" },
                    }

                    div { class: "big small wide short",
                        div {},
                        div {},
                        div {},
                        div {},
                    }
                }
            }

            h2 {}
            h3 {}
            "abcd123"
        }
    })
}
