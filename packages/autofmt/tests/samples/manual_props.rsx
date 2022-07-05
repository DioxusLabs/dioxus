rsx! {
    div {
        Component {
            adsasd: "asd",
            onclick: move |_| {
                let a = a;
            },
            div { "thing" }
        }
        Component {
            asdasd: "asdasd",
            asdasd: "asdasdasdasdasdasdasdasdasdasd",
            ..Props { a: 10, b: 20 }
        }
        Component {
            asdasd: "asdasd",
            ..Props {
                a: 10,
                b: 20,
                c: {
                    fn main() {}
                },
            },
            "content"
        }
    }
}
