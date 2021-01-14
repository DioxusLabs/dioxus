use html_macro::html;
use virtual_node::{IterableNodes, VirtualNode};

#[test]
fn text_root_node() {
    assert_eq!(&html! { some text }.to_string(), "some text");
}

#[test]
fn text_variable_root() {
    let text = "hello world";

    assert_eq!(&html! { { text } }.to_string(), "hello world");
}

#[test]
fn raw_string_literal() {
    assert_eq!(
        &html! { <div>{ r#"Hello World"# }</div> }.to_string(),
        "<div>Hello World</div>"
    );
}

#[test]
fn text_variable_child() {
    let text = "world";

    assert_eq!(
        &html! { <div>{ text }</div> }.to_string(),
        "<div>world</div>"
    )
}

#[test]
fn text_space_after_start_tag() {
    assert_eq!(
        &html! { <div> After Start Tag</div> }.to_string(),
        "<div> After Start Tag</div>"
    )
}

#[test]
fn text_space_before_end_tag() {
    assert_eq!(
        &html! { <div>Before End Tag </div> }.to_string(),
        "<div>Before End Tag </div>"
    )
}

#[test]
fn text_space_before_block() {
    let text = "Before Block";

    assert_eq!(
        &html! { <div> {text}</div> }.to_string(),
        "<div> Before Block</div>"
    )
}

#[test]
fn text_space_after_block() {
    let text = "Hello";

    assert_eq!(
        &html! { <div>{text} </div> }.to_string(),
        "<div>Hello </div>"
    )
}

#[test]
fn text_space_in_block_ignored() {
    let text = "Hello";

    assert_eq!(
        &html! { <div>{ text }</div> }.to_string(),
        "<div>Hello</div>"
    )
}

#[test]
fn text_multiple_text_no_space_between() {
    let hello = "Hello";
    let world = "World";

    assert_eq!(
        &html! { <div>{ hello }{ world }</div> }.to_string(),
        "<div>HelloWorld</div>"
    )
}

#[test]
fn text_multiple_text_space_between() {
    let hello = "Hello";
    let world = "World";

    assert_eq!(
        &html! { <div>{ hello } { world }</div> }.to_string(),
        "<div>Hello World</div>"
    )
}

#[test]
fn text_multiple_text_space_around() {
    let hello = "Hello";
    let world = "World";

    assert_eq!(
        &html! { <div> { hello }{ world } </div> }.to_string(),
        "<div> HelloWorld </div>"
    )
}

#[test]
fn text_multiple_text_space_between_around() {
    let hello = "Hello";
    let world = "World";

    assert_eq!(
        &html! { <div> { hello } { world } </div> }.to_string(),
        "<div> Hello World </div>"
    )
}

#[test]
fn text_tokens_in_between_vars_without_space() {
    let hello = "Hello";
    let world = "World";

    assert_eq!(
        &html! { <div>{ hello }NoSpace{ world }</div> }.to_string(),
        "<div>HelloNoSpaceWorld</div>"
    )
}

#[test]
fn text_tokens_in_between_vars_with_space() {
    let hello = "Hello";
    let world = "World";

    assert_eq!(
        &html! { <div>{ hello } Space { world }</div> }.to_string(),
        "<div>Hello Space World</div>"
    )
}

#[test]
fn text_tokens_in_between_vars_space_around_between() {
    let hello = "Hello";
    let world = "World";

    assert_eq!(
        &html! { <div> { hello } Space { world } </div> }.to_string(),
        "<div> Hello Space World </div>"
    )
}

#[test]
fn text_space_before_next_open_tag() {
    assert_eq!(
        &html! { <div>Hello <img /> world</div> }.to_string(),
        "<div>Hello <img> world</div>"
    )
}

#[test]
fn text_no_space_before_open_tag() {
    assert_eq!(
        &html! { <div>Hello<img /> world</div> }.to_string(),
        "<div>Hello<img> world</div>"
    )
}
