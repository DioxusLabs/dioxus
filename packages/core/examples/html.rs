use bumpalo::Bump;

fn main() {}

fn build(factory: Factory) {
    factory.text();
    div::new(factory)
        .r#class()
        .r#tag()
        .r#type()
        .add_children()
        .iter_children()
        .finish();
}

/// # The `div` element
///
///
/// The <div> HTML element is the generic container for flow content. It has no effect on the content or layout until
/// styled in some way using CSS (e.g. styling is directly applied to it, or some kind of layout model like Flexbox is
/// applied to its parent element).
///
/// As a "pure" container, the <div> element does not inherently represent anything. Instead, it's used to group content
/// so it can be easily styled using the class or id attributes, marking a section of a document as being written in a
/// different language (using the lang attribute), and so on.
///
/// ## Usage
/// ```
/// rsx!{
///     div { class: "tall", id: "unique id"
///         h1 {}
///         p {}
///     }
/// }
///
/// ```
///
/// ## Specifications
/// - Content categories: Flow content, palpable content.
/// - Permitted content: Flow content.
/// - Permitted parents: Any element that accepts flow content.
#[allow(non_camel_case_types)]
struct div {}

struct h1 {}

struct h2 {}

trait BasicElement: Sized {
    const TagName: &'static str;
    fn get_bump(&self) -> &Bump;
    fn new(factory: Factory) -> Self;
    fn add_children(self) -> Self {
        self
    }
    fn iter_children(self) -> Self {
        self
    }
    fn finish(self) -> Self {
        self
    }
}

impl BasicElement for div {
    const TagName: &'static str = "div";

    fn get_bump(&self) -> &Bump {
        todo!()
    }

    fn new(factory: Factory) -> Self {
        todo!()
    }
}

impl div {
    fn class(self) -> Self {
        self
    }
    fn tag(self) -> Self {
        self
    }
    fn r#type(self) -> Self {
        self
    }
}

#[derive(Clone, Copy)]
struct Factory<'a> {
    bump: &'a bumpalo::Bump,
}

impl<'a> Factory<'a> {
    fn text(&self) -> &str {
        todo!()
    }
    fn new_el(&'a self, tag: &'static str) -> ElementBuilder<'a> {
        ElementBuilder {
            bump: self.bump,
            tag,
        }
    }
}

struct ElementBuilder<'a> {
    tag: &'static str,
    bump: &'a bumpalo::Bump,
}
