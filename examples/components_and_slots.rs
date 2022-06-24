use dioxus::prelude::*;

fn main() {
    dioxus::desktop::launch(App.renderer());
}

#[component]
fn App(cx: Scope) -> Element {
    fn simple_name(cx: Scope<NameRenderProps>) -> Element {
        cx.render(dom(|h| {
            [h.element("div")
                .children([h.text("simple name: "), h.text(&cx.props.name)])
                .finish()]
        }))
    }

    let clicked = use_state(&cx, || 0);
    let component_num = *clicked.get() % 3;

    cx.render(dom(move |h| {
        [
            h.element("div")
                .children([h.component(
                    Wrapper,
                    WrapperProps {
                        name: "Joe".into(),
                        slot: match component_num {
                            0 => Slot::from_renderer("my name renderer", simple_name),
                            1 => Slot::from_component(FancyNameRenderer),
                            _ => Slot::empty(),
                        },
                    },
                )])
                .finish(),
            h.element("button")
                .on_any("click", |_| clicked.set(*clicked.get() + 1))
                .children([match component_num {
                    0 => h.text("switch to fancy"),
                    1 => h.text("switch to empty"),
                    _ => h.text("switch to simple"),
                }])
                .finish(),
        ]
    }))
}

#[derive(PartialEq, Props)]
struct WrapperProps {
    name: String,
    slot: Slot<NameRenderProps>,
}

/// Renders a given name with a given component.
#[component]
fn Wrapper(cx: Scope<WrapperProps>) -> Element {
    cx.render(dom(|h| {
        [h.element("div")
            .children([
                h.element("div")
                    .children([h.text("the div content below is rendered by another component:")]),
                h.element("div").children([h.component(
                    cx.props.slot,
                    NameRenderProps {
                        name: cx.props.name.clone(),
                    },
                )]),
            ])
            .finish()]
    }))
}

#[derive(PartialEq, Props)]
struct NameRenderProps {
    name: String,
}

/// Renders the given name in in a formatted way.
#[component]
fn FancyNameRenderer(cx: Scope<NameRenderProps>) -> Element {
    cx.render(dom(|h| {
        [h.element("div")
            .style([
                ("background-color", "#FDECF9"),
                ("border-radius", "5px"),
                ("margin-top", "2px"),
                ("padding", "2px"),
            ])
            .children([
                h.text("fancy name: "),
                h.element("span")
                    .style([("font-style", "italic")])
                    .children([h.text(&cx.props.name)])
                    .finish(),
            ])
            .finish()]
    }))
}

/// An example way of extending the builder with additional functionality.
trait ElementStyleExt: Sized {
    /// Accepts CSS style pairs and sets the `style` attribute for a given element.
    ///
    /// E.g.:
    ///
    /// ```
    /// h.element("div").style([("padding", "1px"), ("background-color", "red")]);
    /// ```
    fn style<I, S>(self, style: I) -> Self
    where
        I: IntoIterator<Item = (S, S)>,
        S: AsRef<str>;
}

impl ElementStyleExt for ElementBuilder<'_> {
    fn style<I, S>(self, style: I) -> Self
    where
        I: IntoIterator<Item = (S, S)>,
        S: AsRef<str>,
    {
        self.attr(
            "style",
            &style
                .into_iter()
                .map(|(k, v)| format!("{}: {}", k.as_ref(), v.as_ref()))
                .collect::<Vec<_>>()
                .join("; "),
        )
    }
}
