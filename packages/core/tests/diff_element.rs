use dioxus::dioxus_core::AttributeValue;
use dioxus::prelude::*;
use dioxus_core::generation;
use dioxus_renderer_oracle::Sequence;

#[test]
fn text_diff() {
    fn app() -> Element {
        let g = generation();
        rsx!( h1 { "hello {g}" } )
    }

    Sequence::new()
        .render_with_expected(app, rsx!( h1 { "hello 0" } ))
        .render_with_expected(app, rsx!( h1 { "hello 1" } ))
        .render_with_expected(app, rsx!( h1 { "hello 2" } ))
        .render_with_expected(app, rsx!( h1 { "hello 3" } ))
        .assert_edit_summary(1, |s| assert_eq!(s.set_texts, 1))
        .assert_edit_summary(2, |s| assert_eq!(s.set_texts, 1))
        .assert_edit_summary(3, |s| assert_eq!(s.set_texts, 1))
        .run();
}

#[test]
fn element_swap() {
    fn app() -> Element {
        let g = generation();

        match g % 2 {
            0 => rsx!( h1 { "hello 1" } ),
            1 => rsx!( h2 { "hello 2" } ),
            _ => unreachable!(),
        }
    }

    Sequence::new()
        .render_with_expected(app, rsx!( h1 { "hello 1" } ))
        .render_with_expected(app, rsx!( h2 { "hello 2" } ))
        .render_with_expected(app, rsx!( h1 { "hello 1" } ))
        .render_with_expected(app, rsx!( h2 { "hello 2" } ))
        .render_with_expected(app, rsx!( h1 { "hello 1" } ))
        .assert_edit_summary(1, |s| assert_eq!(s.replaces, 1))
        .assert_edit_summary(2, |s| assert_eq!(s.replaces, 1))
        .assert_edit_summary(3, |s| assert_eq!(s.replaces, 1))
        .assert_edit_summary(4, |s| assert_eq!(s.replaces, 1))
        .run();
}

#[test]
fn attribute_diff() {
    fn attr(name: &'static str, value: &'static str) -> Attribute {
        Attribute::new(name, AttributeValue::Text(value.into()), None, false)
    }

    fn app() -> Element {
        let g = generation();

        // attributes have to be sorted by name
        let attrs = match g % 5 {
            0 => vec![Attribute::new(
                "a",
                AttributeValue::Text("hello".into()),
                None,
                false,
            )],
            1 => vec![
                Attribute::new("a", AttributeValue::Text("hello".into()), None, false),
                Attribute::new("b", AttributeValue::Text("hello".into()), None, false),
                Attribute::new("c", AttributeValue::Text("hello".into()), None, false),
            ],
            2 => vec![
                Attribute::new("c", AttributeValue::Text("hello".into()), None, false),
                Attribute::new("d", AttributeValue::Text("hello".into()), None, false),
                Attribute::new("e", AttributeValue::Text("hello".into()), None, false),
            ],
            3 => vec![Attribute::new(
                "d",
                AttributeValue::Text("world".into()),
                None,
                false,
            )],
            _ => unreachable!(),
        };

        rsx!(
            div {
                ..attrs,
                "hello"
            }
        )
    }

    fn expected_0() -> Element {
        rsx!( div { ..vec![attr("a", "hello")], "hello" } )
    }

    fn expected_1() -> Element {
        rsx!( div { ..vec![attr("a", "hello"), attr("b", "hello"), attr("c", "hello")], "hello" } )
    }

    fn expected_2() -> Element {
        rsx!( div { ..vec![attr("c", "hello"), attr("d", "hello"), attr("e", "hello")], "hello" } )
    }

    fn expected_3() -> Element {
        rsx!( div { ..vec![attr("d", "world")], "hello" } )
    }

    Sequence::new()
        .render_with_expected(app, expected_0())
        .render_with_expected(app, expected_1())
        .render_with_expected(app, expected_2())
        .render_with_expected(app, expected_3())
        .assert_edit_summary(1, |s| assert_eq!(s.set_attrs, 2))
        .assert_edit_summary(2, |s| assert_eq!(s.set_attrs, 4))
        .assert_edit_summary(3, |s| assert_eq!(s.set_attrs, 3))
        .run();
}

#[test]
fn diff_empty() {
    fn app() -> Element {
        match generation() % 2 {
            0 => rsx! { div { "hello" } },
            1 => rsx! {},
            _ => unreachable!(),
        }
    }

    Sequence::new()
        .render_with_expected(app, rsx! { div { "hello" } })
        .render_with_expected(app, rsx! {})
        .assert_edit_summary(1, |s| assert_eq!(s.replaces, 1))
        .run();
}
