use dioxus::prelude::*;
use dioxus_builder::*;
use dioxus_core::{Attribute, IntoAttributeValue};

const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let count = use_signal(|| 0);

    fragment()
        .child(document::Title(
            document::TitleProps::builder()
                .children(text_node("Dioxus Builder Demo"))
                .build(),
        ))
        .child(document::Stylesheet(
            document::LinkProps::builder()
                .href(Some(TAILWIND_CSS.to_string()))
                .build(),
        ))
        .child(body_section(count))
        .build()
}

fn use_controlled(
    pressed: ReadSignal<Option<bool>>,
    default_pressed: bool,
    on_pressed_change: Callback<bool>,
) -> (Signal<bool>, Callback<bool>) {
    let mut state = use_signal(|| pressed().unwrap_or(default_pressed));

    if let Some(value) = pressed() {
        if state() != value {
            state.set(value);
        }
    }

    let pressed_for_cb = pressed;
    let on_pressed_change = on_pressed_change.clone();
    let mut state_for_cb = state.clone();
    let set_state = Callback::new(move |value: bool| {
        on_pressed_change.call(value);
        if pressed_for_cb().is_none() {
            state_for_cb.set(value);
        }
    });

    (state, set_state)
}

/// The props for the [`Toggle`] component.
#[derive(Props, Clone, PartialEq)]
pub struct ToggleProps {
    /// The controlled pressed state of the toggle.
    pub pressed: ReadSignal<Option<bool>>,

    /// The default pressed state when uncontrolled.
    #[props(default)]
    pub default_pressed: bool,

    /// Whether the toggle is disabled.
    #[props(default)]
    pub disabled: ReadSignal<bool>,

    /// Callback fired when the pressed state changes.
    #[props(default)]
    pub on_pressed_change: Callback<bool>,

    // https://github.com/DioxusLabs/dioxus/issues/2467
    /// Callback fired when the toggle is mounted.
    #[props(default)]
    pub onmounted: Callback<Event<MountedData>>,
    /// Callback fired when the toggle receives focus.
    #[props(default)]
    pub onfocus: Callback<Event<FocusData>>,
    /// Callback fired when a key is pressed on the toggle.
    #[props(default)]
    pub onkeydown: Callback<Event<KeyboardData>>,
    /// Callback fired when the toggle is clicked.
    #[props(default)]
    pub onclick: Callback<MouseEvent>,

    /// Additional attributes to apply to the toggle element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the toggle component.
    pub children: Element,
}

/// # Toggle
///
/// The `Toggle` component is a button that can be on or off.
pub fn Toggle(props: ToggleProps) -> Element {
    let ToggleProps {
        pressed,
        default_pressed,
        disabled,
        on_pressed_change,
        onmounted,
        onfocus,
        onkeydown,
        onclick,
        attributes,
        children,
    } = props;

    let (pressed, set_pressed) = use_controlled(pressed, default_pressed, on_pressed_change);

    let pressed_for_click = pressed.clone();
    let set_pressed = set_pressed.clone();
    let disabled_for_click = disabled;
    let onclick = onclick.clone();

    button()
        .class_list([
            "inline-flex",
            "items-center",
            "justify-center",
            "rounded-md",
            "border",
            "border-gray-300",
            "transition",
        ])
        .class_if(pressed(), "bg-blue-600 text-white")
        .class_if(!pressed(), "bg-white text-gray-800")
        .class_if(disabled(), "opacity-50 cursor-not-allowed")
        .r#type("button")
        .disabled(disabled())
        .attr("aria-pressed", pressed())
        .attr("data-state", if pressed() { "on" } else { "off" })
        .attr("data-disabled", disabled())
        .onmounted(move |event| onmounted.call(event))
        .onfocus(move |event| onfocus.call(event))
        .onkeydown(move |event| onkeydown.call(event))
        .onclick(move |event| {
            if disabled_for_click() {
                return;
            }
            onclick.call(event);
            let new_pressed = !pressed_for_click();
            set_pressed.call(new_pressed);
        })
        .attrs(attributes)
        .child(children)
        .build()
}

#[derive(Clone, Copy)]
pub enum ToggleVariant {
    Solid,
    Outline,
    Ghost,
}

#[derive(Clone, Copy)]
pub enum ToggleSize {
    Sm,
    Md,
    Lg,
}

pub struct ToggleBuilder {
    pressed: ReadSignal<Option<bool>>,
    default_pressed: bool,
    disabled: ReadSignal<bool>,
    on_pressed_change: Callback<bool>,
    onmounted: Callback<Event<MountedData>>,
    onfocus: Callback<Event<FocusData>>,
    onkeydown: Callback<Event<KeyboardData>>,
    onclick: Callback<MouseEvent>,
    attributes: Vec<Attribute>,
    children: Element,
    variant: ToggleVariant,
    size: ToggleSize,
}

impl ToggleBuilder {
    pub fn new() -> Self {
        Self {
            pressed: ReadSignal::default(),
            default_pressed: false,
            disabled: ReadSignal::default(),
            on_pressed_change: Callback::default(),
            onmounted: Callback::default(),
            onfocus: Callback::default(),
            onkeydown: Callback::default(),
            onclick: Callback::default(),
            attributes: Vec::new(),
            children: VNode::empty(),
            variant: ToggleVariant::Solid,
            size: ToggleSize::Md,
        }
    }

    pub fn pressed(mut self, value: ReadSignal<Option<bool>>) -> Self {
        self.pressed = value;
        self
    }

    pub fn default_pressed(mut self, value: bool) -> Self {
        self.default_pressed = value;
        self
    }

    pub fn disabled(mut self, value: ReadSignal<bool>) -> Self {
        self.disabled = value;
        self
    }

    pub fn on_pressed_change(mut self, f: impl FnMut(bool) + 'static) -> Self {
        self.on_pressed_change = Callback::new(f);
        self
    }

    pub fn on_pressed_change_cb(mut self, cb: Callback<bool>) -> Self {
        self.on_pressed_change = cb;
        self
    }

    pub fn onmounted(mut self, f: impl FnMut(Event<MountedData>) + 'static) -> Self {
        self.onmounted = Callback::new(f);
        self
    }

    pub fn onfocus(mut self, f: impl FnMut(Event<FocusData>) + 'static) -> Self {
        self.onfocus = Callback::new(f);
        self
    }

    pub fn onkeydown(mut self, f: impl FnMut(Event<KeyboardData>) + 'static) -> Self {
        self.onkeydown = Callback::new(f);
        self
    }

    pub fn on_click(mut self, f: impl FnMut(MouseEvent) + 'static) -> Self {
        self.onclick = Callback::new(f);
        self
    }

    pub fn on_click_cb(mut self, cb: Callback<MouseEvent>) -> Self {
        self.onclick = cb;
        self
    }

    pub fn label(mut self, text: impl ToString) -> Self {
        self.children = text_node(text);
        self
    }

    pub fn child(mut self, child: impl IntoToggleChild) -> Self {
        self.children = child.into_element();
        self
    }

    pub fn variant(mut self, variant: ToggleVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn outline(self) -> Self {
        self.variant(ToggleVariant::Outline)
    }

    pub fn ghost(self) -> Self {
        self.variant(ToggleVariant::Ghost)
    }

    pub fn size(mut self, size: ToggleSize) -> Self {
        self.size = size;
        self
    }

    pub fn size_sm(self) -> Self {
        self.size(ToggleSize::Sm)
    }

    pub fn size_lg(self) -> Self {
        self.size(ToggleSize::Lg)
    }

    pub fn attr<T>(mut self, name: &'static str, value: impl IntoAttributeValue<T>) -> Self {
        self.attributes
            .push(Attribute::new(name, value, None, false));
        self
    }

    pub fn class(mut self, value: impl IntoAttributeValue) -> Self {
        self.attributes
            .push(Attribute::new("class", value, None, false));
        self
    }

    pub fn class_if(mut self, condition: bool, value: impl IntoAttributeValue) -> Self {
        if condition {
            self.attributes
                .push(Attribute::new("class", value, None, false));
        }
        self
    }

    pub fn class_list<I, S>(mut self, classes: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let joined = classes
            .into_iter()
            .map(|c| c.as_ref().to_string())
            .filter(|c| !c.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        if !joined.is_empty() {
            self.attributes
                .push(Attribute::new("class", joined, None, false));
        }
        self
    }

    pub fn attrs(mut self, attrs: impl IntoIterator<Item = Attribute>) -> Self {
        self.attributes.extend(attrs);
        self
    }

    pub fn build(mut self) -> Element {
        self = self.class_list([
            "inline-flex",
            "items-center",
            "justify-center",
            "rounded-md",
            "transition",
        ]);
        match self.variant {
            ToggleVariant::Solid => {
                self = self.class_list([
                    "bg-blue-600",
                    "text-white",
                    "hover:bg-blue-700",
                    "border",
                    "border-blue-700",
                ]);
            }
            ToggleVariant::Outline => {
                self = self.class_list([
                    "border",
                    "border-gray-300",
                    "text-gray-900",
                    "hover:bg-gray-50",
                ]);
            }
            ToggleVariant::Ghost => {
                self = self.class_list(["text-gray-900", "hover:bg-gray-100"]);
            }
        }
        match self.size {
            ToggleSize::Sm => self = self.class_list(["h-8", "w-8", "text-sm"]),
            ToggleSize::Md => self = self.class_list(["h-10", "w-10", "text-base"]),
            ToggleSize::Lg => self = self.class_list(["h-12", "w-12", "text-lg"]),
        }
        Toggle(ToggleProps {
            pressed: self.pressed,
            default_pressed: self.default_pressed,
            disabled: self.disabled,
            on_pressed_change: self.on_pressed_change,
            onmounted: self.onmounted,
            onfocus: self.onfocus,
            onkeydown: self.onkeydown,
            onclick: self.onclick,
            attributes: self.attributes,
            children: self.children,
        })
    }
}

pub fn toggle() -> ToggleBuilder {
    ToggleBuilder::new()
}

pub trait IntoToggleChild {
    fn into_element(self) -> Element;
}

impl IntoToggleChild for Element {
    fn into_element(self) -> Element {
        self
    }
}

impl IntoToggleChild for ElementBuilder {
    fn into_element(self) -> Element {
        self.build()
    }
}

impl IntoToggleChild for &str {
    fn into_element(self) -> Element {
        text_node(self)
    }
}

impl IntoToggleChild for String {
    fn into_element(self) -> Element {
        text_node(self)
    }
}

fn body_section(count: Signal<i32>) -> Element {
    let mut toggle_pressed = use_signal(|| Some(false));
    let toggle_disabled = use_memo(move || count() % 2 == 0);
    let on_toggle_change = {
        let mut toggle_pressed = toggle_pressed.clone();
        Callback::new(move |value| toggle_pressed.set(Some(value)))
    };

    div()
        .class_list([
            "flex",
            "flex-col",
            "items-center",
            "justify-center",
            "min-h-screen",
            "bg-gray-100",
            "p-8",
            "space-y-6",
        ])
        .child(header_section())
        .child(counter_section(count))
        .child(list_section(count))
        .child(attribute_helpers_section(count))
        .child(toggle_section(
            toggle_pressed.into(),
            toggle_disabled.into(),
            on_toggle_change,
        ))
        .child(footer_section())
        .build()
}

fn header_section() -> Element {
    div()
        .class_list(["container", "mx-auto", "p-4", "text-center", "space-y-2"])
        .child(
            h1().class("text-4xl font-bold text-blue-600")
                .child("Dioxus Builder Demo"),
        )
        .child(
            p().class("text-lg text-gray-700")
                .child("This UI is built using the typed builder API and Tailwind CSS."),
        )
        .build()
}

fn counter_section(mut count: Signal<i32>) -> Element {
    div()
        .class_list(["flex", "space-x-4", "items-center"])
        .child(
            button()
                .class("px-4 py-2 bg-red-500 text-white rounded hover:bg-red-600 transition")
                .onclick(move |_| count -= 1)
                .child("-"),
        )
        .child(
            span()
                .class("text-2xl font-mono w-12 text-center")
                .child(count.to_string()),
        )
        .child(
            button()
                .class("px-4 py-2 bg-green-500 text-white rounded hover:bg-green-600 transition")
                .onclick(move |_| count += 1)
                .child("+"),
        )
        .build()
}

fn list_section(count: Signal<i32>) -> Element {
    div()
        .class("mt-4 w-full max-w-md bg-white shadow-xl rounded-lg overflow-hidden")
        .child(
            div()
                .class("p-4 border-b bg-gray-50")
                .child(h2().class("font-semibold").child("Item List")),
        )
        .child(
            ul().class("divide-y divide-gray-200")
                .children((0..count()).map(|i| {
                    li().class("p-4 hover:bg-gray-50 flex justify-between")
                        .child(span().child(format!("Item record #{}", i + 1)))
                        .child(
                            span()
                                .class("text-xs text-gray-400 capitalize")
                                .child(if i % 2 == 0 { "Even" } else { "Odd" }),
                        )
                })),
        )
        .build()
}

fn attribute_helpers_section(count: Signal<i32>) -> Element {
    let is_even = count() % 2 == 0;
    let extra_attrs = [
        Attribute::new("data-role", "builder-demo", None, false),
        Attribute::new("data-count", count().to_string(), None, false),
    ];

    div()
        .class_list([
            "mt-4",
            "w-full",
            "max-w-md",
            "bg-white",
            "shadow",
            "rounded-lg",
            "p-4",
            "space-y-3",
            "border",
        ])
        .class_if(is_even, "border-green-300")
        .class_if(!is_even, "border-amber-300")
        .attr_if(is_even, "data-state", "even")
        .attrs(extra_attrs)
        .child(h2().class("font-semibold").child("Attribute Helpers"))
        .child(
            p().class("text-sm text-gray-600")
                .child("Uses class_list, class_if, attr_if, and attrs()."),
        )
        .child_if(
            is_even,
            p().class("text-sm text-green-600")
                .child("child_if: count is even"),
        )
        .child_if_else(
            is_even,
            p().class("text-xs text-gray-400")
                .child("child_if_else: even branch"),
            p().class("text-xs text-gray-400")
                .child("child_if_else: odd branch"),
        )
        .build()
}

fn footer_section() -> Element {
    footer()
        .class("mt-8 text-gray-400 text-sm")
        .child("Built with dioxus-builder")
        .build()
}

fn toggle_section(
    pressed: ReadSignal<Option<bool>>,
    disabled: ReadSignal<bool>,
    on_pressed_change: Callback<bool>,
) -> Element {
    div()
        .class_list([
            "mt-4",
            "w-full",
            "max-w-md",
            "bg-white",
            "shadow",
            "rounded-lg",
            "p-4",
        ])
        .child(h2().class("font-semibold").child("Toggle"))
        .child(
            p().class("text-sm text-gray-600")
                .child("Builder component with extendable attributes."),
        )
        .child(
            toggle()
                .pressed(pressed)
                .disabled(disabled)
                .on_pressed_change_cb(on_pressed_change)
                .outline()
                .size_lg()
                .class("shadow-sm")
                .label("B")
                .build(),
        )
        .child(
            p().class("text-xs text-gray-500")
                .child("Disabled when the counter is even."),
        )
        .build()
}
