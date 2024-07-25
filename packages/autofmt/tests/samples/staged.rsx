rsx! {
    div {}

    div { "hi" }

    div { class: "hello", "hi" }

    div { class: "hello", glass: "123", "hi" }

    div { {some_expr} }
    div {
        {
            POSTS.iter().enumerate().map(|(id, post)| rsx! {
                BlogPostItem { post, id }
            })
        }
    }

    div { class: "123123123123123123123123123123123123",
        {some_really_long_expr_some_really_long_expr_some_really_long_expr_some_really_long_expr_}
    }

    div { class: "-my-8 divide-y-2 divide-gray-100",
        {POSTS.iter().enumerate().map(|(id, post)| rsx! { BlogPostItem { post: post, id: id } })}
    }
}
