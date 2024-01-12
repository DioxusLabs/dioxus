rsx! {
    div {
        div {
            div {
                div {
                    section { class: "body-font overflow-hidden dark:bg-ideblack",
                        div { class: "container px-6 mx-auto",
                            div { class: "-my-8 divide-y-2 divide-gray-100",
                                {POSTS.iter().enumerate().map(|(id, post)| rsx! { BlogPostItem { post: post, id: id } })}
                            }
                        }
                    }
                }
            }
        }
    }
}
