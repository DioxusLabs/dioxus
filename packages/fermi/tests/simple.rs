mod basic {

    use dioxus::prelude::*;
    use fermi::*;

    pub static TITLE: Atom<String> = |_| "The Biggest Name in Hollywood".to_string();

    struct Key<'a> {
        words: Vec<&'a str>,
    }

    fn parsed_logs(root: Select) -> Key {
        let name = root.get(TITLE);

        Key {
            words: name.split_ascii_whitespace().collect(),
        }
    }

    fn app(cx: Scope) -> Element {
        let val = use_selector(&cx, parsed_logs);
        let val2 = use_selector(&cx, |s| s.get(TITLE).as_str());

        cx.render(rsx! {
            ul {
                val.words.iter().map(|f| rsx! {
                    li {"{f}"}
                })
            }
        })
    }

    #[test]
    fn it_works() {
        let mut dom = VirtualDom::new(app);
        let v = dom.rebuild();
    }

    #[test]
    fn type_name_is_right() {
        fn get_name<V, O>(s: fn(V) -> O) -> &'static str {
            std::any::type_name::<O>()
        }

        dbg!(get_name(TITLE));
    }
}
