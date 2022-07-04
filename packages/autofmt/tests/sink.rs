use dioxus_autofmt::*;

#[test]
fn formats_valid_rust_src() {
    let src = r#"
//
rsx! {
    div {}
    div {
        h3 {"asd"
        }
    }
}
"#;

    let formatted = fmt_file(src);

    println!("{formatted:#?}");
}

#[test]
fn formats_valid_rust_src_with_indents() {
    let src = r#"
#[inline_props]
fn NavItem<'a>(cx: Scope, to: &'static str, children: Element<'a>, icon: Shape) -> Element {
    const ICON_SIZE: u32 = 36;

    rsx! {
        div { h1 { "thing" } }
    }
}
"#
    .to_string();

    let formatted = fmt_file(&src);

    assert!(formatted.is_empty());
}

#[test]
fn formats_multiple_blocks() {
    let mut src = r#"
#[inline_props]
fn NavItem<'a>(cx: Scope, to: &'static str, children: Element<'a>, icon: Shape) -> Element {
    const ICON_SIZE: u32 = 36;

    rsx! {
        div { h1 { "thing" } }
    }

    rsx! {
        div {
            Ball {
                a: rsx! {
    "asdasd"
}
            }
        }
    }
}
#[inline_props]
fn NavItem<'a>(cx: Scope, to: &'static str, children: Element<'a>, icon: Shape) -> Element {
    const ICON_SIZE: u32 = 36;

    rsx! {
        div { h1 { "thing" } }
    }

    rsx! {
        div {
            Ball {
                a: rsx! {
    "asdasd"
}
            }
        }
    }
}
"#
    .to_string();

    let formatted = fmt_file(&src);

    dbg!(&formatted);

    let block = formatted.into_iter().next().unwrap();

    src.replace_range(
        block.start - 1..block.end + 1,
        &format!("{{ {}    }}", &block.formatted),
    );
}

#[test]
fn empty_blocks() {
    let src = r###"
pub fn Alert(cx: Scope) -> Element {
    cx.render(rsx! {
        div {}
    })
}
"###
    .to_string();

    let formatted = fmt_file(&src);

    dbg!(&formatted);
}
