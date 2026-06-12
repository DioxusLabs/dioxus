#![allow(deprecated)]

fn format_once(src: &str) -> String {
    dioxus_autofmt::apply_formats(src, dioxus_autofmt::fmt_file(src, Default::default()))
        .replace('\r', "")
}

fn assert_already_formatted(src: &str) {
    let src = src.replace('\r', "");
    let once = format_once(&src);
    pretty_assertions::assert_eq!(src, once);
}

#[test]
fn spread_wrappers_with_single_child_are_stable() {
    let src = r#"
fn demo(props: Props, month_year: String) {
    rsx! {
        label { r#for: props.html_for, ..props.attributes,
            {props.children} }
        div { role: "tablist", ..props.attributes,
            {props.children} }
        div { ..props.attributes,
            {month_year} }
    }
}
"#;

    let expected = r#"
fn demo(props: Props, month_year: String) {
    rsx! {
        label { r#for: props.html_for, ..props.attributes,
            {props.children}
        }
        div { role: "tablist", ..props.attributes,
            {props.children}
        }
        div { ..props.attributes,
            {month_year}
        }
    }
}
"#;

    pretty_assertions::assert_eq!(expected, format_once(src));
    pretty_assertions::assert_eq!(expected, format_once(expected));
}

#[test]
fn multiline_call_arguments_keep_normal_rust_indentation() {
    let src = r#"
fn demo() {
    rsx! {
        div {
            {
                let max_date = calculate_next_visible_calendar_month_after_keyboard_navigation(
                    minimum_visible_calendar_date,
                    base_calendar_context.visible_month_count(),
                    enabled_date_range.end(),
                )
                .unwrap_or(minimum_visible_calendar_date);
                max_date
            }
        }
    }
}
"#;

    let out = format_once(src);

    assert!(
        out.contains(
            "let max_date = calculate_next_visible_calendar_month_after_keyboard_navigation(\n                    minimum_visible_calendar_date,\n                    base_calendar_context.visible_month_count(),\n                    enabled_date_range.end(),\n                )"
        ),
        "formatted call arguments were not aligned like normal Rust:\n{out}"
    );
    assert!(
        !out.contains(
            "calculate_next_visible_calendar_month_after_keyboard_navigation(\n                            minimum_visible_calendar_date"
        ),
        "formatter over-indented the first call argument:\n{out}"
    );
}

#[test]
fn multiline_if_attribute_values_indent_the_expression_body() {
    let src = r#"
fn demo(open: bool) {
    rsx! {
        div {
            class:
            if open {
                "dx-docs-sidebar-backdrop dx-docs-sidebar-backdrop-open"
            } else {
                "dx-docs-sidebar-backdrop"
            },
            aria_hidden: "true",
            onclick: close,
        }
    }
}
"#;

    let out = format_once(src);

    assert!(
        out.contains("class:\n                if open {"),
        "multiline attribute value was not indented under the attribute:\n{out}"
    );
}

#[test]
fn empty_rsx_parenthesized_return_stays_compact() {
    assert_already_formatted(
        r#"
fn demo() -> Element {
    return rsx!({});
}
"#,
    );
}
