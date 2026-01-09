use bon::Builder;
use dioxus::dioxus_core::FunctionComponent;
use dioxus::prelude::*;
use dioxus_builder::document::{doc_stylesheet, doc_title};
use dioxus_builder::*;
use dioxus_core::{Attribute, IntoAttributeValue};

const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let count = use_signal(|| 0);

    // Using the new doc_title and doc_stylesheet helpers
    fragment()
        .child(doc_title("Dioxus Builder"))
        .child(doc_stylesheet(TAILWIND_CSS))
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

    /// Additional attributes to apply to the toggle element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the toggle component.
    pub children: Element,
}

/// # Toggle
///
/// The `Toggle` component is a button that can be on or off.
#[allow(non_snake_case)]
pub fn Toggle(props: ToggleProps) -> Element {
    let ToggleProps {
        pressed,
        default_pressed,
        disabled,
        on_pressed_change,
        attributes,
        children,
    } = props;

    let (pressed, set_pressed) = use_controlled(pressed, default_pressed, on_pressed_change);

    let pressed_for_click = pressed.clone();
    let set_pressed = set_pressed.clone();
    let disabled_for_click = disabled;
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
        .onclick(move |_| {
            if disabled_for_click() {
                return;
            }
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

    pub fn label(mut self, text: impl ToString) -> Self {
        self.children = text_node(text);
        self
    }

    pub fn child(mut self, child: Element) -> Self {
        self.children = child;
        self
    }

    pub fn variant(mut self, variant: ToggleVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn outline(self) -> Self {
        self.variant(ToggleVariant::Outline)
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

    fn classes(mut self, classes: &[&'static str]) -> Self {
        for class in classes {
            self = self.class(*class);
        }
        self
    }

    pub fn attrs(mut self, attrs: impl IntoIterator<Item = Attribute>) -> Self {
        self.attributes.extend(attrs);
        self
    }

    pub fn build(mut self) -> Element {
        self = self.classes(&[
            "inline-flex",
            "items-center",
            "justify-center",
            "rounded-md",
            "transition",
        ]);
        match self.variant {
            ToggleVariant::Solid => {
                self = self.classes(&[
                    "bg-blue-600",
                    "text-white",
                    "hover:bg-blue-700",
                    "border",
                    "border-blue-700",
                ]);
            }
            ToggleVariant::Outline => {
                self = self.classes(&[
                    "border",
                    "border-gray-300",
                    "text-gray-900",
                    "hover:bg-gray-50",
                ]);
            }
        }
        match self.size {
            ToggleSize::Sm => self = self.classes(&["h-8", "w-8", "text-sm"]),
            ToggleSize::Md => self = self.classes(&["h-10", "w-10", "text-base"]),
            ToggleSize::Lg => self = self.classes(&["h-12", "w-12", "text-lg"]),
        }
        Toggle(ToggleProps {
            pressed: self.pressed,
            default_pressed: self.default_pressed,
            disabled: self.disabled,
            on_pressed_change: self.on_pressed_change,
            attributes: self.attributes,
            children: self.children,
        })
    }
}

pub fn toggle() -> ToggleBuilder {
    ToggleBuilder::new()
}

// =============================================================================
// Card Component - Demonstrates bon::Builder integration with Props
// =============================================================================

/// A simple card component that demonstrates using bon::Builder with Props.
///
/// When you add `#[props(bon)]`, the Props derive delegates builder generation
/// to bon, giving you a clean builder API without manual implementation.
///
/// Note: `#[props(extends = ...)]` is NOT supported with bon. Use standard
/// Props derive for components that need attribute spreading (like Toggle above).
// Note: bon::Builder MUST come before Props in the derive list!
// This ensures bon generates the builder before Props tries to reference it.
#[derive(Clone, PartialEq, Builder, Props)]
#[props(bon)]
pub struct CardProps {
    /// The title of the card
    #[builder(into)]
    pub title: String,

    /// Optional subtitle - bon auto-defaults Option<T> to None
    #[builder(into)]
    pub subtitle: Option<String>,

    /// Whether the card has a border
    #[builder(default)]
    pub bordered: bool,

    /// The variant/style of the card
    #[builder(default)]
    pub variant: CardVariant,

    /// The children of the card
    pub children: Element,
}

#[derive(Clone, Copy, PartialEq, Default)]
pub enum CardVariant {
    #[default]
    Default,
    Primary,
}

/// A simple Card component using bon-generated props builder.
///
/// Usage with FunctionComponent trait:
/// ```ignore
/// Card.new()
///     .title("My Card")
///     .subtitle("A subtitle")
///     .bordered(true)
///     .variant(CardVariant::Primary)
///     .children(rsx! { "Content" })
///     .build()
/// ```
#[allow(non_snake_case)]
pub fn Card(props: CardProps) -> Element {
    let CardProps {
        title,
        subtitle,
        bordered,
        variant,
        children,
    } = props;

    let bg_class = match variant {
        CardVariant::Default => "bg-white",
        CardVariant::Primary => "bg-blue-50",
    };

    let border_class = match variant {
        CardVariant::Default => "border-gray-200",
        CardVariant::Primary => "border-blue-200",
    };

    div()
        .class_list(["rounded-lg", "shadow-sm", "overflow-hidden", bg_class])
        .class_if(bordered, border_class)
        .class_if(bordered, "border")
        .child(
            div()
                .class("p-4")
                .child(
                    h3().class("text-lg font-semibold text-gray-800")
                        .text(&title),
                )
                .child_option(subtitle.map(|s| p().class("text-sm text-gray-600 mt-1").text(s))),
        )
        .child(div().class("p-4 pt-0").child(children))
        .build()
}

fn body_section(count: Signal<i32>) -> Element {
    let toggle_pressed = use_signal(|| Some(false));
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
        .child(toggle_section(
            toggle_pressed.into(),
            toggle_disabled.into(),
            on_toggle_change,
        ))
        .child(bon_builder_section())
        .child(footer_section())
        .build()
}

fn header_section() -> Element {
    div()
        .class_list(["container", "mx-auto", "p-4", "text-center", "space-y-2"])
        .child(
            h1().class("text-4xl font-bold text-blue-600")
                .text("Dioxus Builder"),
        )
        .child(
            p().class("text-lg text-gray-700")
                .text("A type-safe, fluent API for building UI without rsx!."),
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
                .text("-"),
        )
        .child(
            span()
                .class("text-2xl font-mono w-12 text-center")
                .text(count.to_string()),
        )
        .child(
            button()
                .class("px-4 py-2 bg-green-500 text-white rounded hover:bg-green-600 transition")
                .onclick(move |_| count += 1)
                .text("+"),
        )
        .build()
}

fn list_section(count: Signal<i32>) -> Element {
    let total = count().max(0) as usize;

    div()
        .class("mt-4 w-full max-w-md bg-white shadow-xl rounded-lg overflow-hidden")
        .child(
            div()
                .class("p-4 border-b bg-gray-50")
                .child(h2().class("font-semibold").text("Keyed List")),
        )
        .child(
            // Using the new children_keyed method for efficient list reconciliation
            ul().class("divide-y divide-gray-200").children_keyed(
                0..total,
                |i| i.to_string(),
                |i| {
                    li().class("p-4 hover:bg-gray-50 flex justify-between")
                        .child(span().text(format!("Item record #{}", i + 1)))
                        .child(
                            span()
                                .class("text-xs text-gray-400 capitalize")
                                .text(if i % 2 == 0 { "Even" } else { "Odd" }),
                        )
                },
            ),
        )
        .build()
}

fn footer_section() -> Element {
    footer()
        .class("mt-8 text-gray-400 text-sm")
        .text("Built with dioxus-builder and bon")
        .build()
}

// =============================================================================
// Bon Builder Section - Demonstrates bon::Builder integration
// =============================================================================

/// Demonstrates the bon::Builder integration with Props derive.
///
/// Key features shown:
/// 1. `#[derive(bon::Builder, Props)]` with `#[props(bon)]` for automatic builder
/// 2. `FunctionComponent` trait for `Component.new()` syntax
/// 3. bon attributes like `#[builder(into)]`, `#[builder(default)]`
fn bon_builder_section() -> Element {
    div()
        .class_list([
            "mt-4",
            "w-full",
            "max-w-md",
            "bg-white",
            "shadow-xl",
            "rounded-lg",
            "p-4",
            "space-y-4",
        ])
        .child(
            h2().class("font-semibold text-lg text-purple-800")
                .text("bon::Builder Components"),
        )
        .child(
            p().class("text-sm text-gray-600")
                .text("Use #[derive(bon::Builder, Props)] with #[props(bon)] for a typed builder."),
        )
        // Example 1: Using CardProps::builder() directly
        .child(
            div()
                .class("space-y-2")
                .child(
                    p().class("text-xs font-medium text-gray-500")
                        .text("Method 1: CardProps::builder()"),
                )
                .child(Card(
                    CardProps::builder()
                        .title("Default Card")
                        .children(
                            p().class("text-gray-600")
                                .text("Built from props::builder()")
                                .build(),
                        )
                        .build(),
                )),
        )
        // Example 2: Using FunctionComponent trait - Card.new()
        .child(
            div()
                .class("space-y-2")
                .child(
                    p().class("text-xs font-medium text-gray-500")
                        .text("Method 2: Card.new() via FunctionComponent trait"),
                )
                .child(Card(
                    Card.new()
                        .title("Primary Card")
                        .subtitle("Using Card.new() syntax")
                        .variant(CardVariant::Primary)
                        .bordered(true)
                        .children(
                            p().class("text-gray-600")
                                .text("This uses the FunctionComponent trait!")
                                .build(),
                        )
                        .build(),
                )),
        )
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
