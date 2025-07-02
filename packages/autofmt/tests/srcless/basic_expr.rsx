parse_quote! {
    div {
        "hi"
        {children}
    }
    Fragment {
        Fragment {
            Fragment {
                Fragment {
                    Fragment {
                        div { "Finally have a real node!" }
                    }
                }
            }
        }
    }
    div { class, "hello world" }
    h1 { class, "hello world" }
    h1 { class, {children} }
    h1 { class, id, {children} }
    h1 { class,
        "hello world"
        {children}
    }
    h1 { id,
        "hello world"
        {children}
    }
    Other { class, children }
    Other { class,
        "hello world"
        {children}
    }
    div {
        class: "asdasd",
        onclick: move |_| {
            let a = 10;
            let b = 40;
            let c = 50;
        },
        src1: asset!("/123.png"),
        src2: asset!("/456.png"),
        src3: asset!("/789.png"),
        src4: asset!("/101112.png", WithOptions),
        "hi"
    }
    p {
        img {
            src: asset!("/example-book/assets1/logo.png", AssetOptions::image().with_avif()),
            alt: "some_local1",
            title: "",
        }
        img {
            src: asset!("/example-book/assets2/logo.png", AssetOptions::image().with_avif()),
            alt: "some_local2",
            title: "",
        }
    }
    div { class: "asd", "Jon" }
}
